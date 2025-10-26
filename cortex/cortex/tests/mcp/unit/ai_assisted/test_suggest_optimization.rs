//! Unit Tests for cortex.ai.suggest_optimization
//!
//! Tests cover:
//! - Algorithmic optimizations
//! - Data structure improvements
//! - Clone reduction suggestions
//! - Parallelization opportunities
//! - Estimated speedup calculations
//! - Trade-off analysis

use super::test_helpers::*;
use cortex_mcp::tools::ai_assisted::AiSuggestOptimizationTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_suggest_optimization_nested_loops() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "find_pairs",
        fixtures::NESTED_LOOPS,
        "src/search.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["algorithm"],
        "include_benchmarks": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should suggest algorithmic improvement for nested loops
        let optimizations = data["optimizations"].as_array().unwrap();
        assert!(optimizations.len() > 0, "Should find optimization opportunities");

        let algo_opt = optimizations.iter().find(|o| o["optimization_type"] == "algorithm");
        assert!(algo_opt.is_some(), "Should suggest algorithm optimization");

        // Check optimization structure
        let opt = algo_opt.unwrap();
        assert!(!opt["description"].is_null());
        assert!(!opt["before_code"].is_null());
        assert!(!opt["after_code"].is_null());
        assert!(!opt["reasoning"].is_null());
        assert!(opt["estimated_speedup"].as_f64().unwrap() > 1.0);
    }
}

#[tokio::test]
async fn test_suggest_optimization_excessive_clones() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_strings",
        fixtures::EXCESSIVE_CLONES,
        "src/processor.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["memory", "algorithm"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should suggest reducing clones
        let optimizations = data["optimizations"].as_array().unwrap();
        assert!(optimizations.len() > 0);

        let clone_opt = optimizations.iter().find(|o| o["optimization_type"] == "memory");
        assert!(clone_opt.is_some(), "Should suggest memory optimization for clones");

        let opt = clone_opt.unwrap();
        assert!(opt["description"].as_str().unwrap().contains("clone"));
        assert_eq!(opt["memory_impact"].as_str().unwrap(), "reduced memory allocations");
    }
}

#[tokio::test]
async fn test_suggest_optimization_vec_contains() {
    let fixture = AiAssistedTestFixture::new().await;

    let vec_contains_code = r#"
fn check_membership(items: Vec<i32>, queries: Vec<i32>) -> Vec<bool> {
    let mut results = Vec::new();
    for query in queries {
        results.push(items.contains(&query));
        if items.contains(&query) {
            println!("Found");
        }
        if items.contains(&query) {
            println!("Still found");
        }
    }
    results
}
    "#;

    let unit_id = fixture.create_test_unit(
        "check_membership",
        vec_contains_code,
        "src/check.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["data_structure"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let optimizations = data["optimizations"].as_array().unwrap();
        assert!(optimizations.len() > 0);

        // Should suggest using HashSet instead of Vec
        let ds_opt = optimizations.iter().find(|o| o["optimization_type"] == "data_structure");
        assert!(ds_opt.is_some(), "Should suggest data structure optimization");

        let opt = ds_opt.unwrap();
        assert!(opt["after_code"].as_str().unwrap().contains("HashSet"));
    }
}

#[tokio::test]
async fn test_suggest_optimization_parallelization() {
    let fixture = AiAssistedTestFixture::new().await;

    // Long function with independent iterations
    let parallelizable_code = format!(
        r#"
fn process_items(items: Vec<i32>) -> Vec<i32> {{
    let mut results = Vec::new();
    for item in items {{
        // Imagine long computation here
        {}
        results.push(item * 2);
    }}
    results
}}
        "#,
        "        let x = item * 2;\n".repeat(25)
    );

    let unit_id = fixture.create_test_unit(
        "process_items",
        &parallelizable_code,
        "src/parallel.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["parallelization"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let optimizations = data["optimizations"].as_array().unwrap();

        if optimizations.len() > 0 {
            let para_opt = optimizations.iter().find(|o| o["optimization_type"] == "parallelization");
            if let Some(opt) = para_opt {
                assert!(opt["after_code"].as_str().unwrap().contains("par_iter") ||
                        opt["reasoning"].as_str().unwrap().contains("parallel"));
            }
        }
    }
}

#[tokio::test]
async fn test_suggest_optimization_estimated_speedup() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "find_pairs",
        fixtures::NESTED_LOOPS,
        "src/search.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["algorithm"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Check overall estimated speedup
        assert!(!data["estimated_speedup"].is_null());

        let speedup = data["estimated_speedup"].as_f64().unwrap();
        assert!(speedup >= 1.0, "Estimated speedup should be at least 1.0x");

        // Check individual optimization speedups
        let optimizations = data["optimizations"].as_array().unwrap();
        for opt in optimizations {
            let opt_speedup = opt["estimated_speedup"].as_f64().unwrap();
            assert!(opt_speedup > 0.0, "Each optimization should have positive speedup");
        }
    }
}

#[tokio::test]
async fn test_suggest_optimization_trade_offs() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "find_pairs",
        fixtures::NESTED_LOOPS,
        "src/search.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["algorithm", "data_structure"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let optimizations = data["optimizations"].as_array().unwrap();
        if optimizations.len() > 0 {
            let first = &optimizations[0];

            // Should include trade-off analysis
            assert!(!first["trade_offs"].is_null());
            assert!(!first["memory_impact"].is_null());

            let trade_offs = first["trade_offs"].as_array().unwrap();
            assert!(trade_offs.len() > 0, "Should list trade-offs");
        }
    }
}

#[tokio::test]
async fn test_suggest_optimization_multiple_types() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_strings",
        fixtures::EXCESSIVE_CLONES,
        "src/processor.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["algorithm", "data_structure", "memory", "parallelization"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        assert_eq!(data["total_count"].as_i64().unwrap(), data["optimizations"].as_array().unwrap().len() as i64);
    }
}

#[tokio::test]
async fn test_suggest_optimization_confidence_scores() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "find_pairs",
        fixtures::NESTED_LOOPS,
        "src/search.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["algorithm"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let optimizations = data["optimizations"].as_array().unwrap();
        for opt in optimizations {
            let confidence = opt["confidence"].as_f64().unwrap();
            assert!(confidence >= 0.0 && confidence <= 1.0, "Confidence should be between 0 and 1");
        }
    }
}

#[tokio::test]
async fn test_suggest_optimization_nonexistent_unit() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": "nonexistent-unit-id",
        "optimization_types": ["algorithm"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent unit");
}

#[tokio::test]
async fn test_suggest_optimization_empty_result() {
    let fixture = AiAssistedTestFixture::new().await;

    // Simple, already optimal code
    let optimal_code = r#"
fn get_value() -> i32 {
    42
}
    "#;

    let unit_id = fixture.create_test_unit(
        "get_value",
        optimal_code,
        "src/simple.rs",
        1
    ).await.unwrap();

    let tool = AiSuggestOptimizationTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "optimization_types": ["algorithm", "memory"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should return empty optimizations for already optimal code
        assert_eq!(data["total_count"].as_i64().unwrap(), 0);
        assert_eq!(data["estimated_speedup"].as_f64().unwrap(), 1.0);
    }
}
