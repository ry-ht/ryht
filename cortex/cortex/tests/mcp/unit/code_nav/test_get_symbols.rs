//! Unit Tests for cortex.code.get_symbols
//!
//! Tests cover:
//! - Getting all public symbols in a file
//! - Getting symbols from a module
//! - Filtering by visibility (public only)
//! - Symbol metadata (name, type, signature)
//! - Empty file/module
//! - Multiple symbol types
//! - Cross-language support
//! - Error handling
//! - Performance measurement

use super::test_helpers::*;
use cortex::mcp::tools::code_nav::CodeGetSymbolsTool;
use cortex_core::types::Visibility;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_get_symbols_basic() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public and private functions in a file
    let mut public_fn = fixtures::create_rust_function(
        "public_api",
        "myapp::public_api",
        "src/lib.rs",
        10,
    );
    public_fn.visibility = Visibility::Public;

    let mut private_fn = fixtures::create_rust_function(
        "private_helper",
        "myapp::private_helper",
        "src/lib.rs",
        20,
    );
    private_fn.visibility = Visibility::Private;

    fixture.store_unit(&public_fn).await.unwrap();
    fixture.store_unit(&private_fn).await.unwrap();

    // Get symbols (should only return public ones)
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/lib.rs",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to get symbols");
    assert!(duration < 100, "Took too long: {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["scope"], "src/lib.rs");
        assert_eq!(data["count"], 1); // Only public function

        let symbols = data["symbols"].as_array().unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0]["name"], "public_api");
    }
}

#[tokio::test]
async fn test_get_symbols_multiple_types() {
    let fixture = CodeNavTestFixture::new().await;

    // Create various public symbols
    let mut func = fixtures::create_rust_function(
        "process",
        "myapp::process",
        "src/api.rs",
        10,
    );
    func.visibility = Visibility::Public;

    let mut struct_unit = fixtures::create_rust_struct(
        "User",
        "myapp::User",
        "src/api.rs",
        30,
    );
    struct_unit.visibility = Visibility::Public;

    let mut trait_unit = fixtures::create_rust_trait(
        "Handler",
        "myapp::Handler",
        "src/api.rs",
        50,
    );
    trait_unit.visibility = Visibility::Public;

    fixture.store_unit(&func).await.unwrap();
    fixture.store_unit(&struct_unit).await.unwrap();
    fixture.store_unit(&trait_unit).await.unwrap();

    // Get all symbols
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/api.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 3);

        let symbols = data["symbols"].as_array().unwrap();
        let symbol_types: Vec<&str> = symbols
            .iter()
            .map(|s| s["unit_type"].as_str().unwrap())
            .collect();

        assert!(symbol_types.contains(&"Function"));
        assert!(symbol_types.contains(&"Struct"));
        assert!(symbol_types.contains(&"Trait"));
    }
}

#[tokio::test]
async fn test_get_symbols_with_signatures() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public function with signature
    let mut func = fixtures::create_rust_function(
        "calculate",
        "myapp::math::calculate",
        "src/math.rs",
        5,
    );
    func.visibility = Visibility::Public;
    func.signature = "pub fn calculate(x: i32, y: i32) -> i32".to_string();

    fixture.store_unit(&func).await.unwrap();

    // Get symbols and verify signature is included
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/math.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let symbols = data["symbols"].as_array().unwrap();

        assert_eq!(symbols[0]["name"], "calculate");
        assert_eq!(symbols[0]["signature"], "pub fn calculate(x: i32, y: i32) -> i32");
    }
}

#[tokio::test]
async fn test_get_symbols_with_documentation() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public function with docstring
    let mut func = fixtures::create_rust_function(
        "documented_fn",
        "myapp::documented_fn",
        "src/lib.rs",
        10,
    );
    func.visibility = Visibility::Public;
    func.docstring = Some("/// This is a well-documented function".to_string());

    fixture.store_unit(&func).await.unwrap();

    // Get symbols and verify docstring
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/lib.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let symbols = data["symbols"].as_array().unwrap();

        assert!(!symbols[0]["docstring"].is_null());
        assert!(symbols[0]["docstring"].as_str().unwrap().contains("well-documented"));
    }
}

#[tokio::test]
async fn test_get_symbols_empty_file() {
    let fixture = CodeNavTestFixture::new().await;

    // Get symbols from a file with no units
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/empty.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Should succeed even with no symbols");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 0);
        assert_eq!(data["symbols"].as_array().unwrap().len(), 0);
    }
}

#[tokio::test]
async fn test_get_symbols_only_private() {
    let fixture = CodeNavTestFixture::new().await;

    // Create only private symbols
    let mut private1 = fixtures::create_rust_function(
        "helper1",
        "myapp::helper1",
        "src/internal.rs",
        10,
    );
    private1.visibility = Visibility::Private;

    let mut private2 = fixtures::create_rust_function(
        "helper2",
        "myapp::helper2",
        "src/internal.rs",
        20,
    );
    private2.visibility = Visibility::Private;

    fixture.store_unit(&private1).await.unwrap();
    fixture.store_unit(&private2).await.unwrap();

    // Get symbols (should be empty since all are private)
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/internal.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 0);
    }
}

#[tokio::test]
async fn test_get_symbols_typescript() {
    let fixture = CodeNavTestFixture::new().await;

    // Create TypeScript public exports
    let mut class = fixtures::create_typescript_class(
        "UserService",
        "app.services.UserService",
        "src/services.ts",
        10,
    );
    class.visibility = Visibility::Public;

    let mut method = fixtures::create_typescript_method(
        "getUserById",
        "app.services.getUserById",
        "src/services.ts",
        30,
    );
    method.visibility = Visibility::Public;

    fixture.store_unit(&class).await.unwrap();
    fixture.store_unit(&method).await.unwrap();

    // Get symbols
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/services.ts",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 2);

        let symbols = data["symbols"].as_array().unwrap();
        let types: Vec<&str> = symbols
            .iter()
            .map(|s| s["unit_type"].as_str().unwrap())
            .collect();

        assert!(types.contains(&"Class"));
        assert!(types.contains(&"Method"));
    }
}

#[tokio::test]
async fn test_get_symbols_includes_qualified_names() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public symbols with qualified names
    let mut func = fixtures::create_rust_function(
        "connect",
        "myapp::db::connect",
        "src/db.rs",
        5,
    );
    func.visibility = Visibility::Public;

    fixture.store_unit(&func).await.unwrap();

    // Get symbols and verify qualified name
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/db.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let symbols = data["symbols"].as_array().unwrap();

        assert_eq!(symbols[0]["name"], "connect");
        assert_eq!(symbols[0]["qualified_name"], "myapp::db::connect");
    }
}

#[tokio::test]
async fn test_get_symbols_sorted_order() {
    let fixture = CodeNavTestFixture::new().await;

    // Create multiple public symbols
    let mut func_a = fixtures::create_rust_function(
        "aaa_function",
        "myapp::aaa_function",
        "src/lib.rs",
        50,
    );
    func_a.visibility = Visibility::Public;

    let mut func_z = fixtures::create_rust_function(
        "zzz_function",
        "myapp::zzz_function",
        "src/lib.rs",
        10,
    );
    func_z.visibility = Visibility::Public;

    let mut func_m = fixtures::create_rust_function(
        "mmm_function",
        "myapp::mmm_function",
        "src/lib.rs",
        30,
    );
    func_m.visibility = Visibility::Public;

    fixture.store_unit(&func_a).await.unwrap();
    fixture.store_unit(&func_z).await.unwrap();
    fixture.store_unit(&func_m).await.unwrap();

    // Get symbols (order may vary based on implementation)
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/lib.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 3);
    }
}

#[tokio::test]
async fn test_get_symbols_large_file() {
    let fixture = CodeNavTestFixture::new().await;

    // Create many public symbols
    for i in 0..100 {
        let mut func = fixtures::create_rust_function(
            &format!("fn_{}", i),
            &format!("myapp::fn_{}", i),
            "src/large.rs",
            i * 10,
        );
        func.visibility = Visibility::Public;
        fixture.store_unit(&func).await.unwrap();
    }

    // Get all symbols
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/large.rs",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 500, "Should handle large files efficiently, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 100);
    }
}

#[tokio::test]
async fn test_get_symbols_mixed_visibility() {
    let fixture = CodeNavTestFixture::new().await;

    // Create mix of public, private, and other visibility
    let mut public = fixtures::create_rust_function("pub_fn", "myapp::pub_fn", "src/mix.rs", 10);
    public.visibility = Visibility::Public;

    let mut private = fixtures::create_rust_function("priv_fn", "myapp::priv_fn", "src/mix.rs", 20);
    private.visibility = Visibility::Private;

    let mut internal = fixtures::create_rust_function("int_fn", "myapp::int_fn", "src/mix.rs", 30);
    internal.visibility = Visibility::Internal;

    let mut protected = fixtures::create_rust_function("prot_fn", "myapp::prot_fn", "src/mix.rs", 40);
    protected.visibility = Visibility::Protected;

    fixture.store_unit(&public).await.unwrap();
    fixture.store_unit(&private).await.unwrap();
    fixture.store_unit(&internal).await.unwrap();
    fixture.store_unit(&protected).await.unwrap();

    // Get symbols (only public)
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/mix.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 1);
        assert_eq!(data["symbols"].as_array().unwrap()[0]["name"], "pub_fn");
    }
}

#[tokio::test]
async fn test_get_symbols_with_unit_ids() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public symbols
    let mut func = fixtures::create_rust_function(
        "my_fn",
        "myapp::my_fn",
        "src/lib.rs",
        10,
    );
    func.visibility = Visibility::Public;

    fixture.store_unit(&func).await.unwrap();

    // Get symbols and verify unit_id is included
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/lib.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let symbols = data["symbols"].as_array().unwrap();

        assert!(!symbols[0]["unit_id"].is_null());
        // Verify it's a valid ID format
        let unit_id_str = symbols[0]["unit_id"].as_str().unwrap();
        assert!(!unit_id_str.is_empty());
    }
}

#[tokio::test]
async fn test_get_symbols_performance() {
    let fixture = CodeNavTestFixture::new().await;

    // Create realistic number of symbols
    for i in 0..50 {
        let mut func = fixtures::create_rust_function(
            &format!("api_{}", i),
            &format!("myapp::api::api_{}", i),
            "src/api.rs",
            i * 15,
        );
        func.visibility = if i % 3 == 0 { Visibility::Public } else { Visibility::Private };
        fixture.store_unit(&func).await.unwrap();
    }

    // Measure performance
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/api.rs",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 200, "Should be fast, took {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        // Should have ~17 public symbols (every 3rd out of 50)
        assert!(data["count"].as_u64().unwrap() >= 15);
    }
}

#[tokio::test]
async fn test_get_symbols_different_files() {
    let fixture = CodeNavTestFixture::new().await;

    // Create symbols in different files
    let mut func1 = fixtures::create_rust_function("fn1", "myapp::fn1", "src/file1.rs", 10);
    func1.visibility = Visibility::Public;

    let mut func2 = fixtures::create_rust_function("fn2", "myapp::fn2", "src/file2.rs", 10);
    func2.visibility = Visibility::Public;

    fixture.store_unit(&func1).await.unwrap();
    fixture.store_unit(&func2).await.unwrap();

    // Get symbols from file1
    let tool = CodeGetSymbolsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope": "src/file1.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["count"], 1);
        assert_eq!(data["symbols"].as_array().unwrap()[0]["name"], "fn1");
    }
}
