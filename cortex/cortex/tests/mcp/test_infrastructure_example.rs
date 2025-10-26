//! Example tests demonstrating the MCP test infrastructure
//!
//! This file shows how to use:
//! - TestHarness for setup
//! - Assertions for validation
//! - TokenCounter for efficiency measurement
//! - Fixtures for test data

mod utils;

use utils::{
    TestHarness,
    ToolResultAssertions,
    TokenCounter,
    ProjectFixture,
    LanguageType,
    assert_token_efficiency,
};
use mcp_sdk::Tool;
use serde_json::json;

#[tokio::test]
async fn example_workspace_creation_with_assertions() {
    // Setup test harness
    let harness = TestHarness::new().await;

    // Create a test project
    let fixture = ProjectFixture::new(LanguageType::Rust, "example-project");
    let project_path = fixture.write_to(harness.temp_path()).await.unwrap();

    // Get workspace context
    let ctx = harness.workspace_context();
    let tool = cortex_cli::mcp::tools::workspace::WorkspaceCreateTool::new(ctx);

    // Execute tool
    let input = json!({
        "name": "ExampleWorkspace",
        "root_path": project_path.to_string_lossy(),
        "auto_import": true,
        "process_code": true,
    });

    let tool_ctx = mcp_sdk::prelude::ToolContext::default();
    let result = tool.execute(input, &tool_ctx).await.unwrap();

    // Use assertion helpers
    result
        .assert_success()
        .assert_has_field("workspace_id")
        .assert_has_field("files_imported")
        .assert_has_field("units_extracted")
        .assert_field_in_range("files_imported", 2.0, 20.0)
        .assert_field_in_range("units_extracted", 1.0, 100.0);

    // Additional checks
    let json = result.as_json();
    assert!(json["workspace_id"].is_string());
    assert_eq!(json["workspace_type"], "code");

    println!("Workspace created successfully: {}", json["workspace_id"]);
}

#[tokio::test]
async fn example_token_efficiency_measurement() {
    // Setup
    let mut counter = TokenCounter::new();

    // Scenario 1: Finding all functions
    // Traditional: grep + read 50 files (~30,000 tokens)
    // Cortex: semantic search (~70 tokens)
    counter.add_measurement(
        "Find all functions",
        "Search",
        30_000,
        70,
    );

    // Scenario 2: Modify specific method
    // Traditional: read file + write file (~1,200 tokens)
    // Cortex: update_unit (~100 tokens)
    counter.add_measurement(
        "Modify specific method",
        "Manipulation",
        1_200,
        100,
    );

    // Scenario 3: Analyze dependencies
    // Traditional: parse all files (~10,000 tokens)
    // Cortex: get dependency graph (~150 tokens)
    counter.add_measurement(
        "Analyze dependencies",
        "Analysis",
        10_000,
        150,
    );

    // Print report
    counter.print_summary();

    // Verify efficiency
    let total = counter.total_savings();
    assert_token_efficiency(
        total.traditional_tokens,
        total.cortex_tokens,
        75.0, // Minimum 75% savings
    );

    // Export report
    let report = counter.generate_report();
    println!("\nMarkdown report:");
    report.print_markdown();
}

#[tokio::test]
async fn example_fixture_usage() {
    let harness = TestHarness::new().await;

    // Create different language projects
    let rust_project = ProjectFixture::new(LanguageType::Rust, "rust-example");
    let ts_project = ProjectFixture::new(LanguageType::TypeScript, "ts-example");

    // Write to temp directory
    let rust_path = rust_project.write_to(harness.temp_path()).await.unwrap();
    let ts_path = ts_project.write_to(harness.temp_path()).await.unwrap();

    println!("Created Rust project at: {:?}", rust_path);
    println!("Created TypeScript project at: {:?}", ts_path);

    // Both projects can now be used for testing
    assert!(rust_path.join("Cargo.toml").exists());
    assert!(ts_path.join("package.json").exists());
}

#[tokio::test]
async fn example_complete_workflow() {
    // Setup
    let mut harness = TestHarness::new().await;

    // Create and load a project
    let fixture = ProjectFixture::new(LanguageType::Rust, "workflow-example");
    let project_path = fixture.write_to(harness.temp_path()).await.unwrap();

    // Create workspace
    let workspace = harness
        .create_test_workspace("WorkflowTest", &project_path)
        .await;

    println!("Created workspace: {} ({})", workspace.name, workspace.id);

    // Load project
    let load_result = harness.load_project(workspace.id, &project_path).await;

    println!(
        "Loaded {} files, extracted {} units in {} ms",
        load_result.files_loaded,
        load_result.units_extracted,
        load_result.duration_ms
    );

    // Record metrics
    harness.record_metric("Load project", load_result.duration_ms, 0);

    // Traditional approach would need to read all files
    let traditional_tokens = load_result.files_loaded * 600; // ~600 tokens per file
    harness.record_traditional_tokens(traditional_tokens);

    // Cortex approach: just workspace creation
    harness.record_cortex_tokens(100);

    // Print summary
    harness.print_summary();

    assert!(harness.token_savings_percent() > 75.0);
}

#[tokio::test]
async fn example_semantic_search_matcher() {
    use utils::assertions::SemanticSearchMatcher;

    // Simulated search results
    let results = json!([
        {
            "id": "1",
            "name": "add_function",
            "relevance_score": 0.95
        },
        {
            "id": "2",
            "name": "subtract_function",
            "relevance_score": 0.87
        },
        {
            "id": "3",
            "name": "multiply_function",
            "relevance_score": 0.82
        }
    ]);

    // Use the matcher
    let matcher = SemanticSearchMatcher::new(&results);

    matcher
        .min_results(2)
        .min_relevance(0.8)
        .sorted_by_relevance();

    println!("Semantic search validation passed!");
}

#[tokio::test]
async fn example_dependency_matcher() {
    use utils::assertions::DependencyMatcher;

    // Simulated dependency results
    let deps = json!([
        {
            "from": "main.rs",
            "to": "models.rs",
            "dep_type": "import"
        },
        {
            "from": "main.rs",
            "to": "utils.rs",
            "dep_type": "import"
        },
        {
            "from": "models.rs",
            "to": "utils.rs",
            "dep_type": "import"
        }
    ]);

    // Use the matcher
    let matcher = DependencyMatcher::new(&deps);

    matcher
        .min_dependencies(2)
        .contains_dependency("main", "models")
        .no_circular();

    println!("Dependency analysis validation passed!");
}

#[tokio::test]
async fn example_assertion_builder() {
    use utils::assertions::AssertionBuilder;
    use mcp_sdk::prelude::ToolResult;
    use mcp_sdk::content::Content;

    // Create a mock result
    let result = ToolResult {
        content: vec![Content::text(r#"{
            "status": "success",
            "count": 5,
            "items": ["a", "b", "c"]
        }"#)],
        is_error: None,
    };

    // Use the builder for fluent assertions
    AssertionBuilder::new(&result)
        .success()
        .has_field("status")
        .has_field("count")
        .field_equals("status", &json!("success"))
        .array_min_len("items", 3)
        .assert(
            |json| json["count"].as_u64().unwrap() > 0,
            "count should be greater than 0"
        );

    println!("Builder assertions passed!");
}
