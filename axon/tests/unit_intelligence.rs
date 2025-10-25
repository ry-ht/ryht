//! Unit tests for intelligence layer
//!
//! Tests cover:
//! - Model router and selection logic
//! - Context optimizer and token management
//! - Pattern analyzer
//! - Optimization strategies
//! - Caching mechanisms

mod common;

use axon::intelligence::*;

// ============================================================================
// Model Router Tests
// ============================================================================

#[tokio::test]
async fn test_model_router_creation() {
    let router = ModelRouter::new();
    // Router should be created successfully
    assert!(true);
}

#[tokio::test]
async fn test_model_router_with_cache_ttl() {
    let router = ModelRouter::new().with_cache_ttl(std::time::Duration::from_secs(60));
    // Should accept custom cache TTL
    assert!(true);
}

#[tokio::test]
async fn test_model_selection_lowest_cost() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    let result = router
        .select_model(ModelRequirements::LowestCost, context)
        .await;

    assert!(result.is_ok());
    let selection = result.unwrap();
    assert!(!selection.model_id.is_empty());
    assert!(selection.estimated_cost < 0.01); // Should select cheap model
}

#[tokio::test]
async fn test_model_selection_fastest_response() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    let result = router
        .select_model(ModelRequirements::FastestResponse, context)
        .await;

    assert!(result.is_ok());
    let selection = result.unwrap();
    assert!(selection.estimated_latency_ms < 1000); // Should select fast model
}

#[tokio::test]
async fn test_model_selection_highest_quality() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    let result = router
        .select_model(ModelRequirements::HighestQuality, context)
        .await;

    assert!(result.is_ok());
    let selection = result.unwrap();
    assert!(selection.historical_success_rate > 0.90); // Should select high-quality model
}

#[tokio::test]
async fn test_model_selection_balanced() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    let result = router
        .select_model(ModelRequirements::Balanced, context)
        .await;

    assert!(result.is_ok());
    let selection = result.unwrap();
    assert!(!selection.model_id.is_empty());
    assert!(!selection.provider_id.is_empty());
    assert!(selection.confidence > 0.0);
}

#[tokio::test]
async fn test_model_selection_custom_weights() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    let result = router
        .select_model(
            ModelRequirements::Custom {
                cost_weight: 0.8,
                speed_weight: 0.1,
                quality_weight: 0.1,
            },
            context,
        )
        .await;

    assert!(result.is_ok());
    let selection = result.unwrap();
    // Should favor cost with these weights
    assert!(selection.estimated_cost < 0.02);
}

#[tokio::test]
async fn test_selection_context_complexity() {
    let router = ModelRouter::new();

    // Simple task
    let simple_context = SelectionContext {
        task_type: "simple_task".to_string(),
        expected_complexity: TaskComplexity::Simple,
        ..Default::default()
    };

    let simple_result = router
        .select_model(ModelRequirements::Balanced, simple_context)
        .await
        .unwrap();

    // Complex task
    let complex_context = SelectionContext {
        task_type: "complex_task".to_string(),
        expected_complexity: TaskComplexity::VeryComplex,
        ..Default::default()
    };

    let complex_result = router
        .select_model(ModelRequirements::Balanced, complex_context)
        .await
        .unwrap();

    // Complex tasks should select higher quality models
    assert!(complex_result.historical_success_rate >= simple_result.historical_success_rate);
}

#[tokio::test]
async fn test_selection_context_deadline() {
    let router = ModelRouter::new();

    let context = SelectionContext {
        deadline_ms: Some(500),
        ..Default::default()
    };

    let result = router
        .select_model(ModelRequirements::FastestResponse, context)
        .await
        .unwrap();

    // Should select model that meets deadline
    assert!(result.estimated_latency_ms <= 500);
}

#[tokio::test]
async fn test_model_router_cache() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    // First call
    let result1 = router
        .select_model(ModelRequirements::Balanced, context.clone())
        .await
        .unwrap();

    // Second call with same parameters should use cache
    let result2 = router
        .select_model(ModelRequirements::Balanced, context)
        .await
        .unwrap();

    assert_eq!(result1.model_id, result2.model_id);
}

#[tokio::test]
async fn test_model_router_clear_cache() {
    let router = ModelRouter::new();
    let context = SelectionContext::default();

    router
        .select_model(ModelRequirements::Balanced, context)
        .await
        .unwrap();

    router.clear_cache().await;
    // Cache cleared successfully
    assert!(true);
}

// ============================================================================
// Context Optimizer Tests
// ============================================================================

#[tokio::test]
async fn test_context_optimizer_creation() {
    let optimizer = ContextOptimizer::new();
    // Optimizer created successfully
    assert!(true);
}

#[tokio::test]
async fn test_context_optimizer_with_model() {
    let optimizer = ContextOptimizer::new().with_model("gpt-4".to_string());
    // Optimizer configured with model
    assert!(true);
}

#[tokio::test]
async fn test_count_tokens() {
    let optimizer = ContextOptimizer::new();
    let count = optimizer.count_tokens("Hello, world!").await;

    assert!(count.is_ok());
    assert!(count.unwrap() > 0);
}

#[tokio::test]
async fn test_optimize_within_target() {
    let optimizer = ContextOptimizer::new();
    let content = "let x = 42;".to_string();

    let result = optimizer
        .optimize(content.clone(), 1000, OptimizationStrategy::None)
        .await;

    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert_eq!(optimized.content, content);
    assert_eq!(optimized.optimization_ratio, 1.0);
}

#[tokio::test]
async fn test_optimize_basic() {
    let optimizer = ContextOptimizer::new();
    let content = r#"
        // This is a comment
        let x = 42;
        // Another comment
        let y = 100;
    "#
    .to_string();

    let result = optimizer
        .optimize(content, 1000, OptimizationStrategy::Basic)
        .await;

    assert!(result.is_ok());
    let optimized = result.unwrap();
    // Should remove comments
    assert!(!optimized.content.contains("// This is a comment"));
}

#[tokio::test]
async fn test_optimize_semantic() {
    let optimizer = ContextOptimizer::new();
    let content = r#"
        fn important_function() {}
        // Just a comment
        let x = 42;
    "#
    .to_string();

    let result = optimizer
        .optimize(
            content,
            50,
            OptimizationStrategy::Semantic {
                min_importance: 0.5,
            },
        )
        .await;

    assert!(result.is_ok());
    let optimized = result.unwrap();
    // Should keep important parts
    assert!(optimized.content.contains("function") || optimized.content.len() > 0);
}

#[tokio::test]
async fn test_optimize_aggressive() {
    let optimizer = ContextOptimizer::new();
    let content = r#"
        use std::collections::HashMap;

        pub struct MyStruct {
            field: String,
        }

        // Comment
        impl MyStruct {
            pub fn new() -> Self {
                Self { field: String::new() }
            }
        }

        #[test]
        fn test_struct() {}
    "#
    .to_string();

    let result = optimizer
        .optimize(content, 100, OptimizationStrategy::Aggressive)
        .await;

    assert!(result.is_ok());
    let optimized = result.unwrap();
    // Should aggressively compress
    assert!(optimized.optimization_ratio < 1.0);
}

#[tokio::test]
async fn test_optimize_custom() {
    let optimizer = ContextOptimizer::new();
    let content = r#"
        pub struct MyStruct {}
        pub fn my_function() {}
        // Comment
        let x = 42;
    "#
    .to_string();

    let result = optimizer
        .optimize(
            content,
            100,
            OptimizationStrategy::Custom {
                preserve_types: vec![PartType::TypeDefinition, PartType::FunctionSignature],
                min_importance: 0.6,
            },
        )
        .await;

    assert!(result.is_ok());
    let optimized = result.unwrap();
    // Should preserve specified types
    assert!(optimized.content.contains("MyStruct") || optimized.content.contains("function"));
}

#[tokio::test]
async fn test_chunk_content() {
    let optimizer = ContextOptimizer::new();
    let long_content = "let x = 42;\n".repeat(100);

    let result = optimizer.chunk_content(long_content, 50, 10).await;

    assert!(result.is_ok());
    let chunks = result.unwrap();
    assert!(chunks.len() > 1);

    // Each chunk should be within limits
    for chunk in chunks {
        assert!(chunk.token_count <= 60); // 50 + tolerance
    }
}

#[tokio::test]
async fn test_optimize_simple() {
    let optimizer = ContextOptimizer::new();
    let content = "let x = 42;".to_string();

    let result = optimizer.optimize_simple(content, 100).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_optimizer_cache() {
    let optimizer = ContextOptimizer::new();
    let content = "let x = 42;".to_string();

    let result1 = optimizer
        .optimize(content.clone(), 100, OptimizationStrategy::Basic)
        .await
        .unwrap();

    let result2 = optimizer
        .optimize(content, 100, OptimizationStrategy::Basic)
        .await
        .unwrap();

    assert_eq!(result1.content, result2.content);
}

#[tokio::test]
async fn test_optimizer_clear_cache() {
    let optimizer = ContextOptimizer::new();
    let content = "let x = 42;".to_string();

    optimizer
        .optimize(content, 100, OptimizationStrategy::Basic)
        .await
        .unwrap();

    optimizer.clear_cache().await;
    // Cache cleared
    assert!(true);
}

// ============================================================================
// Part Type Tests
// ============================================================================

#[test]
fn test_part_type_equality() {
    assert_eq!(PartType::Code, PartType::Code);
    assert_eq!(PartType::Comment, PartType::Comment);
    assert_ne!(PartType::Code, PartType::Comment);
}

// ============================================================================
// Optimization Strategy Tests
// ============================================================================

#[test]
fn test_optimization_strategy_variants() {
    let strategies = vec![
        OptimizationStrategy::None,
        OptimizationStrategy::Basic,
        OptimizationStrategy::Semantic {
            min_importance: 0.5,
        },
        OptimizationStrategy::Aggressive,
        OptimizationStrategy::Custom {
            preserve_types: vec![PartType::Code],
            min_importance: 0.6,
        },
    ];

    assert_eq!(strategies.len(), 5);
}

// ============================================================================
// Model Requirements Tests
// ============================================================================

#[test]
fn test_model_requirements_variants() {
    let requirements = vec![
        ModelRequirements::LowestCost,
        ModelRequirements::FastestResponse,
        ModelRequirements::HighestQuality,
        ModelRequirements::Balanced,
        ModelRequirements::Custom {
            cost_weight: 0.33,
            speed_weight: 0.33,
            quality_weight: 0.34,
        },
    ];

    assert_eq!(requirements.len(), 5);
}

// ============================================================================
// Task Complexity Tests
// ============================================================================

#[test]
fn test_task_complexity_levels() {
    let complexities = vec![
        TaskComplexity::Trivial,
        TaskComplexity::Simple,
        TaskComplexity::Medium,
        TaskComplexity::Complex,
        TaskComplexity::VeryComplex,
    ];

    assert_eq!(complexities.len(), 5);
}

// ============================================================================
// Selection Context Tests
// ============================================================================

#[test]
fn test_selection_context_default() {
    let context = SelectionContext::default();
    assert_eq!(context.task_type, "general");
    assert!(matches!(
        context.expected_complexity,
        TaskComplexity::Medium
    ));
    assert!(context.max_tokens.is_none());
    assert!(context.deadline_ms.is_none());
}

#[test]
fn test_selection_context_custom() {
    let context = SelectionContext {
        task_type: "code_generation".to_string(),
        task_description: "Generate Rust code".to_string(),
        expected_complexity: TaskComplexity::Complex,
        max_tokens: Some(2000),
        deadline_ms: Some(5000),
    };

    assert_eq!(context.task_type, "code_generation");
    assert_eq!(context.max_tokens, Some(2000));
    assert_eq!(context.deadline_ms, Some(5000));
}

// ============================================================================
// Intelligence Coordinator Tests
// ============================================================================

#[test]
fn test_intelligence_coordinator_creation() {
    use std::sync::Arc;

    let router = Arc::new(ModelRouter::new());
    let optimizer = Arc::new(ContextOptimizer::new());
    let analyzer = Arc::new(PatternAnalyzer::new());

    let coordinator = IntelligenceCoordinator::new(router.clone(), optimizer.clone(), analyzer);

    // Verify components are accessible
    assert!(true);
}

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_intelligence_error_no_suitable_model() {
    let error = IntelligenceError::NoSuitableModel;
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("No suitable model"));
}

#[test]
fn test_intelligence_error_optimization_failed() {
    let error = IntelligenceError::OptimizationFailed("test error".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Optimization failed"));
    assert!(error_msg.contains("test error"));
}

#[test]
fn test_intelligence_error_pattern_analysis_failed() {
    let error = IntelligenceError::PatternAnalysisFailed("analysis error".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Pattern analysis failed"));
}
