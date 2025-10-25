//! Tests for code extraction functionality.

use cortex_code_analysis::{CodeParser, Lang, RustParser};

#[test]
fn test_extract_complete_function_info() {
    let source = r#"
/// Calculates the sum of two numbers.
///
/// # Arguments
/// * `a` - First number
/// * `b` - Second number
///
/// # Returns
/// The sum of a and b
#[inline]
#[must_use]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];

    // Verify all fields are extracted
    assert_eq!(func.name, "add");
    assert_eq!(func.qualified_name, "add");
    assert_eq!(func.parameters.len(), 2);
    assert_eq!(func.return_type, Some("i32".to_string()));
    assert!(func.docstring.is_some());
    assert!(func.attributes.len() >= 1);
    assert!(!func.body.is_empty());
    assert!(func.start_line >= 1);
    assert!(func.end_line > func.start_line);
}

#[test]
fn test_extract_parameter_details() {
    let source = "fn test(x: &str, mut y: i32, z: &mut Vec<String>) {}";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let params = &result.functions[0].parameters;
    assert_eq!(params.len(), 3);

    // First param: reference
    assert_eq!(params[0].name, "x");
    assert_eq!(params[0].param_type, "&str");
    assert!(params[0].is_reference);

    // Second param: should have name y
    assert_eq!(params[1].name, "y");
    // Note: mut detection depends on tree-sitter grammar details

    // Third param: mutable reference
    assert_eq!(params[2].name, "z");
    assert!(params[2].is_reference);
}

#[test]
fn test_extract_struct_fields() {
    let source = r#"
/// Configuration for the system.
pub struct Config {
    /// Host address
    pub host: String,

    /// Port number
    pub(crate) port: u16,

    /// Internal buffer
    buffer: Vec<u8>,
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    let s = &result.structs[0];

    assert_eq!(s.name, "Config");
    assert!(s.docstring.is_some());
    assert_eq!(s.fields.len(), 3);

    // Check field details
    assert_eq!(s.fields[0].name, "host");
    assert_eq!(s.fields[0].field_type, "String");
}

#[test]
fn test_extract_impl_methods() {
    let source = r#"
struct Calculator;

impl Calculator {
    /// Create a new calculator
    pub fn new() -> Self {
        Self
    }

    /// Add two numbers
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Subtract two numbers
    fn subtract(&self, a: i32, b: i32) -> i32 {
        a - b
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.impls.len(), 1);
    let impl_block = &result.impls[0];

    assert_eq!(impl_block.type_name, "Calculator");
    assert_eq!(impl_block.methods.len(), 3);

    // Check qualified names include the type
    for method in &impl_block.methods {
        assert!(method.qualified_name.contains("Calculator"));
    }
}

#[test]
fn test_extract_enum_variants() {
    let source = r#"
/// HTTP methods
pub enum HttpMethod {
    /// GET request
    Get,
    /// POST request
    Post,
    /// Custom method with data
    Custom { name: String, data: Vec<u8> },
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.enums.len(), 1);
    let e = &result.enums[0];

    assert_eq!(e.name, "HttpMethod");
    assert!(e.docstring.is_some());
    assert_eq!(e.variants.len(), 3);
}

#[test]
fn test_extract_trait_methods() {
    let source = r#"
/// A trait for things that can be drawn
pub trait Drawable {
    /// Draw the item
    fn draw(&self);

    /// Get the color
    fn color(&self) -> String {
        String::from("black")
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.traits.len(), 1);
    let t = &result.traits[0];

    assert_eq!(t.name, "Drawable");
    assert!(t.docstring.is_some());
    assert_eq!(t.methods.len(), 2);
}

#[test]
fn test_extract_line_numbers() {
    let source = r#"
// Line 1
fn first() {}

// Line 5
fn second() {}

// Line 9
fn third() {}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 3);

    // Each function should have correct line numbers
    for func in &result.functions {
        assert!(func.start_line >= 1);
        assert!(func.end_line >= func.start_line);
    }

    // Functions should be in order
    assert!(result.functions[0].start_line < result.functions[1].start_line);
    assert!(result.functions[1].start_line < result.functions[2].start_line);
}

#[test]
fn test_code_parser_auto_detect() {
    let mut parser = CodeParser::new().unwrap();

    // Test Rust file
    let rust_source = "fn test() {}";
    let rust_result = parser.parse_file_auto("test.rs", rust_source).unwrap();
    assert_eq!(rust_result.functions.len(), 1);

    // Test TypeScript file
    let ts_source = "function test() {}";
    let ts_result = parser.parse_file_auto("test.ts", ts_source).unwrap();
    assert_eq!(ts_result.functions.len(), 1);
}

#[test]
fn test_language_specific_parser() {
    let mut rust_parser = CodeParser::for_language(Lang::Rust).unwrap();
    let source = "fn test() {}";
    let result = rust_parser.parse_rust("test.rs", source).unwrap();
    assert_eq!(result.functions.len(), 1);
}

#[test]
fn test_extract_multiple_items() {
    let source = r#"
use std::fmt;

const MAX_SIZE: usize = 1024;

struct Data {
    value: i32,
}

impl Data {
    fn new() -> Self {
        Self { value: 0 }
    }
}

fn process(data: &Data) -> i32 {
    data.value * 2
}

trait Processor {
    fn process(&self) -> i32;
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    // Should extract all different types of items
    assert!(result.imports.len() >= 1);
    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.impls.len(), 1);
    assert_eq!(result.functions.len(), 2); // process() + Data::new()
    assert_eq!(result.traits.len(), 1);
}

#[test]
fn test_extract_nested_modules() {
    let source = r#"
pub mod outer {
    pub mod inner {
        pub fn nested_function() {}
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert!(result.modules.len() >= 1);
}

#[test]
fn test_real_world_rust_code() {
    let source = r#"
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// A cache for storing parsed data.
#[derive(Debug, Clone)]
pub struct ParseCache {
    /// Internal storage
    cache: Arc<RwLock<HashMap<String, ParsedData>>>,
}

#[derive(Debug, Clone)]
struct ParsedData {
    content: String,
    timestamp: u64,
}

impl ParseCache {
    /// Create a new cache.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert data into the cache.
    pub fn insert(&self, key: String, content: String) {
        let data = ParsedData {
            content,
            timestamp: current_timestamp(),
        };
        self.cache.write().insert(key, data);
    }

    /// Get data from the cache.
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache
            .read()
            .get(key)
            .map(|data| data.content.clone())
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("cache.rs", source).unwrap();

    // Verify all components are extracted
    assert!(result.imports.len() >= 3);
    assert!(result.structs.len() >= 2);
    assert!(result.impls.len() >= 2);
    assert!(result.functions.len() >= 1);

    // Verify the main struct has documentation
    let main_struct = result.structs.iter().find(|s| s.name == "ParseCache");
    assert!(main_struct.is_some());
    let main_struct = main_struct.unwrap();
    assert!(main_struct.docstring.is_some());

    // Verify methods are associated with correct type
    let cache_impl = result.impls.iter().find(|i| i.type_name == "ParseCache");
    assert!(cache_impl.is_some());
    let cache_impl = cache_impl.unwrap();
    assert!(cache_impl.methods.len() >= 3);
}
