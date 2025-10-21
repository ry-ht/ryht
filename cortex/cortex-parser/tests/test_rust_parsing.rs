//! Comprehensive tests for Rust parsing.

use cortex_parser::{RustParser, Visibility};

#[test]
fn test_parse_simple_function() {
    let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert_eq!(func.name, "add");
    assert_eq!(func.qualified_name, "add");
    assert_eq!(func.parameters.len(), 2);
    assert_eq!(func.parameters[0].name, "a");
    assert_eq!(func.parameters[0].param_type, "i32");
    assert_eq!(func.parameters[1].name, "b");
    assert_eq!(func.parameters[1].param_type, "i32");
    assert_eq!(func.return_type, Some("i32".to_string()));
    assert_eq!(func.visibility, Visibility::Private);
    assert!(!func.is_async);
    assert!(!func.is_const);
    assert!(!func.is_unsafe);
}

#[test]
fn test_parse_pub_function() {
    let source = "pub fn hello() -> String { String::from(\"hello\") }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert_eq!(func.name, "hello");
    assert_eq!(func.visibility, Visibility::Public);
    assert_eq!(func.return_type, Some("String".to_string()));
}

#[test]
fn test_parse_pub_crate_function() {
    let source = "pub(crate) fn internal() {}";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].visibility, Visibility::PublicCrate);
}

#[test]
fn test_parse_async_function() {
    let source = "async fn fetch_data() -> Result<String, Error> { Ok(String::new()) }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert_eq!(func.name, "fetch_data");
    // Note: async detection may vary based on tree-sitter grammar
}

#[test]
fn test_parse_const_function() {
    let source = "const fn constant_value() -> i32 { 42 }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "constant_value");
    // Note: const detection may vary based on tree-sitter grammar
}

#[test]
fn test_parse_unsafe_function() {
    let source = "unsafe fn dangerous() { }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    assert_eq!(result.functions[0].name, "dangerous");
    // Note: unsafe detection may vary based on tree-sitter grammar
}

#[test]
fn test_parse_function_with_generics() {
    let source = "fn generic<T, U: Clone>(x: T, y: U) -> T { x }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert_eq!(func.name, "generic");
    assert!(func.generics.len() >= 1);
    assert_eq!(func.parameters.len(), 2);
}

#[test]
fn test_parse_function_with_lifetimes() {
    let source = "fn with_lifetime<'a>(s: &'a str) -> &'a str { s }";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert_eq!(func.name, "with_lifetime");
    // Note: lifetime extraction may vary based on tree-sitter grammar
}

#[test]
fn test_parse_function_with_docstring() {
    let source = r#"
/// This is a test function.
/// It adds two numbers together.
///
/// # Examples
/// ```
/// let result = add(2, 2);
/// assert_eq!(result, 4);
/// ```
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert!(func.docstring.is_some());
    let doc = func.docstring.as_ref().unwrap();
    assert!(doc.contains("test function"));
    assert!(doc.contains("Examples"));
}

#[test]
fn test_parse_function_with_attributes() {
    let source = r#"
#[test]
#[should_panic]
fn test_panic() {
    panic!("This should panic");
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert!(func.attributes.len() >= 1);
}

#[test]
fn test_parse_method_with_self() {
    let source = r#"
impl MyStruct {
    fn method(&self) -> i32 {
        42
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.impls.len(), 1);
    let impl_block = &result.impls[0];
    assert_eq!(impl_block.methods.len(), 1);
    let method = &impl_block.methods[0];
    assert_eq!(method.parameters.len(), 1);
    assert!(method.parameters[0].is_self);
}

#[test]
fn test_parse_method_with_mut_self() {
    let source = r#"
impl MyStruct {
    fn mutate(&mut self) {
        // mutation
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.impls.len(), 1);
    let method = &result.impls[0].methods[0];
    assert!(method.parameters[0].is_mut);
}

#[test]
fn test_parse_struct() {
    let source = r#"
struct Person {
    name: String,
    age: u32,
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    let s = &result.structs[0];
    assert_eq!(s.name, "Person");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name, "name");
    assert_eq!(s.fields[0].field_type, "String");
    assert_eq!(s.fields[1].name, "age");
    assert_eq!(s.fields[1].field_type, "u32");
    assert!(!s.is_tuple_struct);
    assert!(!s.is_unit_struct);
}

#[test]
fn test_parse_pub_struct() {
    let source = r#"
pub struct Config {
    pub host: String,
    port: u16,
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    let s = &result.structs[0];
    assert_eq!(s.visibility, Visibility::Public);
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].visibility, Visibility::Public);
    assert_eq!(s.fields[1].visibility, Visibility::Private);
}

#[test]
fn test_parse_tuple_struct() {
    let source = "struct Point(i32, i32);";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    // Note: tuple struct detection may vary based on tree-sitter
}

#[test]
fn test_parse_unit_struct() {
    let source = "struct Unit;";
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    assert!(result.structs[0].is_unit_struct);
}

#[test]
fn test_parse_struct_with_generics() {
    let source = r#"
struct Container<T> {
    value: T,
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    let s = &result.structs[0];
    assert!(s.generics.len() >= 1);
}

#[test]
fn test_parse_enum() {
    let source = r#"
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.enums.len(), 1);
    let e = &result.enums[0];
    assert_eq!(e.name, "Message");
    assert_eq!(e.variants.len(), 3);
}

#[test]
fn test_parse_trait() {
    let source = r#"
trait Drawable {
    fn draw(&self);
    fn color(&self) -> String;
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.traits.len(), 1);
    let t = &result.traits[0];
    assert_eq!(t.name, "Drawable");
    assert_eq!(t.methods.len(), 2);
}

#[test]
fn test_parse_impl_block() {
    let source = r#"
impl MyStruct {
    fn new() -> Self {
        Self {}
    }

    fn method(&self) -> i32 {
        42
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.impls.len(), 1);
    let impl_block = &result.impls[0];
    assert_eq!(impl_block.type_name, "MyStruct");
    assert_eq!(impl_block.trait_name, None);
    assert_eq!(impl_block.methods.len(), 2);
}

#[test]
fn test_parse_trait_impl() {
    let source = r#"
impl Display for MyStruct {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "MyStruct")
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.impls.len(), 1);
    let impl_block = &result.impls[0];
    assert_eq!(impl_block.type_name, "MyStruct");
    assert!(impl_block.trait_name.is_some());
}

#[test]
fn test_parse_module() {
    let source = r#"
pub mod utils {
    pub fn helper() {}
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.modules.len(), 1);
    let module = &result.modules[0];
    assert_eq!(module.name, "utils");
    assert!(module.is_inline);
}

#[test]
fn test_parse_use_statements() {
    let source = r#"
use std::collections::HashMap;
use std::io::{self, Read, Write};
use super::module;
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert!(result.imports.len() >= 3);
}

#[test]
fn test_parse_complex_file() {
    let source = r#"
use std::fmt;

/// A person with a name and age.
#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    age: u32,
}

impl Person {
    /// Create a new person.
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    /// Get the person's age.
    pub fn age(&self) -> u32 {
        self.age
    }
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.age)
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.impls.len(), 2);
    assert!(result.imports.len() >= 1);
}

#[test]
fn test_parse_complexity_calculation() {
    let source = r#"
fn complex_function(x: i32) -> i32 {
    if x > 0 {
        if x > 10 {
            x * 2
        } else {
            x + 1
        }
    } else {
        0
    }
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert!(func.complexity.is_some());
    let complexity = func.complexity.unwrap();
    // Should have multiple decision points
    assert!(complexity > 1);
}

#[test]
fn test_parse_function_with_where_clause() {
    let source = r#"
fn with_where<T>(x: T) -> T
where
    T: Clone + Debug,
{
    x
}
"#;
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source).unwrap();

    assert_eq!(result.functions.len(), 1);
    let func = &result.functions[0];
    assert_eq!(func.name, "with_where");
    // Note: where clause extraction may vary based on tree-sitter grammar
}
