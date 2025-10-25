//! Comprehensive tests for language parsers
//!
//! This test suite validates:
//! - Parser creation and initialization for each language
//! - Basic parsing functionality across all supported languages
//! - Language detection from file extensions
//! - Edge cases and error handling

use cortex_code_analysis::{
    CodeParser, Lang, RustParser, TypeScriptParser,
};

// ============================================================================
// SECTION 1: Parser Initialization Tests
// ============================================================================

#[test]
fn test_rust_parser_creation() {
    let parser = RustParser::new();
    assert!(parser.is_ok());
}

#[test]
fn test_typescript_parser_creation() {
    let parser = TypeScriptParser::new();
    assert!(parser.is_ok());
}

#[test]
fn test_javascript_parser_creation() {
    let parser = TypeScriptParser::new_javascript();
    assert!(parser.is_ok());
}

#[test]
fn test_code_parser_creation() {
    let parser = CodeParser::new();
    assert!(parser.is_ok());
}

#[test]
fn test_code_parser_for_rust() {
    let parser = CodeParser::for_language(Lang::Rust);
    assert!(parser.is_ok());
}

#[test]
fn test_code_parser_for_typescript() {
    let parser = CodeParser::for_language(Lang::TypeScript);
    assert!(parser.is_ok());
}

#[test]
fn test_code_parser_for_javascript() {
    let parser = CodeParser::for_language(Lang::JavaScript);
    assert!(parser.is_ok());
}

// ============================================================================
// SECTION 2: Rust Parser Tests
// ============================================================================

#[test]
fn test_rust_parse_simple_function() {
    let source = r#"
fn hello() {
    println!("Hello, world!");
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "hello");
}

#[test]
fn test_rust_parse_struct() {
    let source = r#"
struct Point {
    x: i32,
    y: i32,
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.structs.len(), 1);
    assert_eq!(parsed.structs[0].name, "Point");
    assert_eq!(parsed.structs[0].fields.len(), 2);
}

#[test]
fn test_rust_parse_enum() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.enums.len(), 1);
    assert_eq!(parsed.enums[0].name, "Color");
}

#[test]
fn test_rust_parse_trait() {
    let source = r#"
trait Drawable {
    fn draw(&self);
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.traits.len(), 1);
    assert_eq!(parsed.traits[0].name, "Drawable");
}

#[test]
fn test_rust_parse_impl() {
    let source = r#"
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.impls.len(), 1);
    assert_eq!(parsed.structs.len(), 1);
}

#[test]
fn test_rust_parse_module() {
    let source = r#"
mod utils {
    pub fn helper() {}
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.modules.len(), 1);
    assert_eq!(parsed.modules[0].name, "utils");
}

#[test]
fn test_rust_parse_generics() {
    let source = r#"
fn generic_function<T>(value: T) -> T {
    value
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "generic_function");
}

#[test]
fn test_rust_parse_async_function() {
    let source = r#"
async fn fetch_data() -> Result<String, Error> {
    Ok("data".to_string())
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "fetch_data");
}

// ============================================================================
// SECTION 3: TypeScript Parser Tests
// ============================================================================

#[test]
fn test_typescript_parse_function() {
    let source = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "greet");
}

#[test]
fn test_typescript_parse_arrow_function() {
    let source = r#"
const add = (a: number, b: number): number => a + b;
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
}

#[test]
fn test_typescript_parse_class() {
    let source = r#"
class Person {
    name: string;
    age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }

    greet(): string {
        return `Hello, I'm ${this.name}`;
    }
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
    // Note: TypeScript classes may be parsed as structs in the current implementation
}

#[test]
fn test_typescript_parse_interface() {
    let source = r#"
interface User {
    id: number;
    name: string;
    email: string;
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
    // Note: Interfaces may be parsed as traits in the current implementation
}

#[test]
fn test_typescript_parse_type_alias() {
    let source = r#"
type Point = {
    x: number;
    y: number;
};
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
    // Note: Type aliases may be parsed as structs in the current implementation
}

#[test]
fn test_typescript_parse_enum() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
    // Note: TypeScript enum parsing might not be fully supported yet
    // This test just verifies the parser doesn't crash
}

#[test]
fn test_typescript_parse_generics() {
    let source = r#"
function identity<T>(arg: T): T {
    return arg;
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
}

// ============================================================================
// SECTION 4: JavaScript Parser Tests
// ============================================================================

#[test]
fn test_javascript_parse_function() {
    let source = r#"
function greet(name) {
    return `Hello, ${name}!`;
}
"#;

    let mut parser = TypeScriptParser::new_javascript().unwrap();
    let result = parser.parse_file("test.js", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].name, "greet");
}

#[test]
fn test_javascript_parse_class() {
    let source = r#"
class Counter {
    constructor() {
        this.count = 0;
    }

    increment() {
        this.count++;
    }
}
"#;

    let mut parser = TypeScriptParser::new_javascript().unwrap();
    let result = parser.parse_file("test.js", source);

    assert!(result.is_ok());
    // Note: JavaScript classes may be parsed as structs in the current implementation
}

#[test]
fn test_javascript_parse_arrow_function() {
    let source = r#"
const multiply = (a, b) => a * b;
"#;

    let mut parser = TypeScriptParser::new_javascript().unwrap();
    let result = parser.parse_file("test.js", source);

    assert!(result.is_ok());
}

#[test]
fn test_javascript_parse_async_function() {
    let source = r#"
async function fetchData() {
    const response = await fetch('/api/data');
    return response.json();
}
"#;

    let mut parser = TypeScriptParser::new_javascript().unwrap();
    let result = parser.parse_file("test.js", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
}

// ============================================================================
// SECTION 5: CodeParser Integration Tests
// ============================================================================

#[test]
fn test_code_parser_parse_rust() {
    let mut parser = CodeParser::new().unwrap();
    let source = "fn test() {}";
    let result = parser.parse_rust("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
}

#[test]
fn test_code_parser_parse_typescript() {
    let mut parser = CodeParser::new().unwrap();
    let source = "function test() {}";
    let result = parser.parse_typescript("test.ts", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
}

#[test]
fn test_code_parser_parse_javascript() {
    let mut parser = CodeParser::new().unwrap();
    let source = "function test() {}";
    let result = parser.parse_javascript("test.js", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 1);
}

#[test]
fn test_code_parser_auto_detect_rust() {
    let mut parser = CodeParser::new().unwrap();
    let source = "fn test() {}";
    let result = parser.parse_file_auto("test.rs", source);

    assert!(result.is_ok());
}

#[test]
fn test_code_parser_auto_detect_typescript() {
    let mut parser = CodeParser::new().unwrap();
    let source = "function test() {}";
    let result = parser.parse_file_auto("test.ts", source);

    assert!(result.is_ok());
}

#[test]
fn test_code_parser_auto_detect_javascript() {
    let mut parser = CodeParser::new().unwrap();
    let source = "function test() {}";
    let result = parser.parse_file_auto("test.js", source);

    assert!(result.is_ok());
}

// ============================================================================
// SECTION 6: Language Detection Tests
// ============================================================================

#[test]
fn test_lang_from_path_rust() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.rs")), Some(Lang::Rust));
}

#[test]
fn test_lang_from_path_typescript() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.ts")), Some(Lang::TypeScript));
}

#[test]
fn test_lang_from_path_tsx() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.tsx")), Some(Lang::Tsx));
}

#[test]
fn test_lang_from_path_javascript() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.js")), Some(Lang::JavaScript));
}

#[test]
fn test_lang_from_path_jsx() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.jsx")), Some(Lang::Jsx));
}

#[test]
fn test_lang_from_path_python() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.py")), Some(Lang::Python));
}

#[test]
fn test_lang_from_path_c() {
    use std::path::Path;

    // C files are mapped to Cpp in the current implementation
    assert_eq!(Lang::from_path(Path::new("test.c")), Some(Lang::Cpp));
}

#[test]
fn test_lang_from_path_cpp() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.cpp")), Some(Lang::Cpp));
}

#[test]
fn test_lang_from_path_java() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.java")), Some(Lang::Java));
}

#[test]
fn test_lang_from_path_unknown() {
    use std::path::Path;

    assert_eq!(Lang::from_path(Path::new("test.unknown")), None);
}

// ============================================================================
// SECTION 7: Error Handling Tests
// ============================================================================

#[test]
fn test_parse_invalid_rust_syntax() {
    let source = r#"
fn broken {
    this is not valid rust
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    // Should still parse but may have errors
    assert!(result.is_ok());
}

#[test]
fn test_parse_empty_file() {
    let source = "";

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.functions.len(), 0);
}

#[test]
fn test_parse_whitespace_only() {
    let source = "   \n   \n   ";

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
}

// ============================================================================
// SECTION 8: Complex Code Pattern Tests
// ============================================================================

#[test]
fn test_rust_parse_nested_modules() {
    let source = r#"
mod outer {
    pub mod inner {
        pub fn helper() {}
    }
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert!(parsed.modules.len() >= 1);
}

#[test]
fn test_rust_parse_multiple_impls() {
    let source = r#"
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
"#;

    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_file("test.rs", source);

    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.impls.len(), 2);
}

#[test]
fn test_typescript_parse_namespace() {
    let source = r#"
namespace Utils {
    export function helper() {
        return "help";
    }
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
}

#[test]
fn test_typescript_parse_decorator() {
    let source = r#"
function Component(target: any) {
    return target;
}

@Component
class MyComponent {
    render() {
        return "rendered";
    }
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let result = parser.parse_file("test.ts", source);

    assert!(result.is_ok());
}
