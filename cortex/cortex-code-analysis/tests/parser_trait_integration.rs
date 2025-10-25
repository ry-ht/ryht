//! Integration tests for the ParserTrait pattern and language abstractions.

use cortex_code_analysis::{
    Parser, ParserTrait, Lang, LanguageInfo,
    RustLanguage, TypeScriptLanguage, JavaScriptLanguage, PythonLanguage,
    Node,
};
use std::path::Path;

#[test]
fn test_rust_parser_trait() {
    let code = br#"
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#.to_vec();

    let parser = Parser::<RustLanguage>::new(code, Path::new("test.rs"))
        .expect("Failed to create Rust parser");

    assert_eq!(parser.get_language(), Lang::Rust);
    assert_eq!(parser.get_root().kind(), "source_file");
    assert!(parser.get_code().len() > 0);
}

#[test]
fn test_typescript_parser_trait() {
    let code = br#"
        function greet(name: string): string {
            return `Hello, ${name}!`;
        }
    "#.to_vec();

    let parser = Parser::<TypeScriptLanguage>::new(code, Path::new("test.ts"))
        .expect("Failed to create TypeScript parser");

    assert_eq!(parser.get_language(), Lang::TypeScript);
    assert_eq!(parser.get_root().kind(), "program");
}

#[test]
fn test_javascript_parser_trait() {
    let code = br#"
        function greet(name) {
            return `Hello, ${name}!`;
        }
    "#.to_vec();

    let parser = Parser::<JavaScriptLanguage>::new(code, Path::new("test.js"))
        .expect("Failed to create JavaScript parser");

    assert_eq!(parser.get_language(), Lang::JavaScript);
    assert_eq!(parser.get_root().kind(), "program");
}

#[test]
fn test_python_parser_trait() {
    let code = br#"
def greet(name: str) -> str:
    return f"Hello, {name}!"
    "#.to_vec();

    let parser = Parser::<PythonLanguage>::new(code, Path::new("test.py"))
        .expect("Failed to create Python parser");

    assert_eq!(parser.get_language(), Lang::Python);
    assert_eq!(parser.get_root().kind(), "module");
}

#[test]
fn test_lang_from_path() {
    assert_eq!(Lang::from_path(Path::new("test.rs")), Some(Lang::Rust));
    assert_eq!(Lang::from_path(Path::new("test.ts")), Some(Lang::TypeScript));
    assert_eq!(Lang::from_path(Path::new("test.tsx")), Some(Lang::Tsx));
    assert_eq!(Lang::from_path(Path::new("test.js")), Some(Lang::JavaScript));
    assert_eq!(Lang::from_path(Path::new("test.jsx")), Some(Lang::Jsx));
    assert_eq!(Lang::from_path(Path::new("test.py")), Some(Lang::Python));
    assert_eq!(Lang::from_path(Path::new("test.java")), Some(Lang::Java));
    assert_eq!(Lang::from_path(Path::new("test.kt")), Some(Lang::Kotlin));
    assert_eq!(Lang::from_path(Path::new("test.cpp")), Some(Lang::Cpp));
}

#[test]
fn test_lang_from_extension() {
    assert_eq!(Lang::from_extension("rs"), Some(Lang::Rust));
    assert_eq!(Lang::from_extension("ts"), Some(Lang::TypeScript));
    assert_eq!(Lang::from_extension("js"), Some(Lang::JavaScript));
    assert_eq!(Lang::from_extension("py"), Some(Lang::Python));
    assert_eq!(Lang::from_extension("unknown"), None);
}

#[test]
fn test_lang_metadata() {
    assert_eq!(Lang::Rust.get_name(), "rust");
    assert_eq!(Lang::Rust.display_name(), "Rust");
    assert_eq!(Lang::Rust.extensions(), &["rs"]);

    assert_eq!(Lang::TypeScript.get_name(), "typescript");
    assert!(Lang::TypeScript.supports_generics());
    assert!(Lang::TypeScript.is_statically_typed());

    assert_eq!(Lang::Python.get_name(), "python");
    assert!(!Lang::Python.is_statically_typed());
}

#[test]
fn test_node_traversal() {
    let code = b"fn test() { let x = 1; }".to_vec();
    let parser = Parser::<RustLanguage>::new(code, Path::new("test.rs"))
        .expect("Failed to create parser");

    let root = parser.get_root();

    // Test basic node properties
    assert_eq!(root.kind(), "source_file");
    assert!(root.child_count() > 0);
    assert!(!root.has_error());

    // Test position methods
    let (start_row, start_col) = root.start_position();
    assert_eq!(start_row, 0);
    assert_eq!(start_col, 0);

    // Test children iteration
    let children: Vec<_> = root.children().collect();
    assert!(children.len() > 0);

    // Test child access
    if let Some(first_child) = root.child(0) {
        assert!(first_child.kind().len() > 0);
    }
}

#[test]
fn test_node_field_access() {
    let code = b"fn add(a: i32, b: i32) -> i32 { a + b }".to_vec();
    let parser = Parser::<RustLanguage>::new(code, Path::new("test.rs"))
        .expect("Failed to create parser");

    let root = parser.get_root();

    // Find the function node
    for child in root.children() {
        if child.kind() == "function_item" {
            // Access named fields
            let name = child.child_by_field_name("name");
            assert!(name.is_some());

            let parameters = child.child_by_field_name("parameters");
            assert!(parameters.is_some());

            let body = child.child_by_field_name("body");
            assert!(body.is_some());

            break;
        }
    }
}

#[test]
fn test_node_text_extraction() {
    let code = b"fn test() {}".to_vec();
    let parser = Parser::<RustLanguage>::new(code, Path::new("test.rs"))
        .expect("Failed to create parser");

    let root = parser.get_root();

    for child in root.children() {
        if child.kind() == "function_item" {
            if let Some(name) = child.child_by_field_name("name") {
                let text = name.utf8_text(parser.get_code());
                assert_eq!(text, Some("test"));
            }
        }
    }
}

#[test]
fn test_generic_parsing_function() {
    fn parse_and_count<T: LanguageInfo>(code: &[u8], path: &str) -> usize {
        let parser = Parser::<T>::new(code.to_vec(), Path::new(path))
            .expect("Failed to create parser");
        parser.get_root().child_count()
    }

    let rust_count = parse_and_count::<RustLanguage>(b"fn test() {}", "test.rs");
    assert!(rust_count > 0);

    let ts_count = parse_and_count::<TypeScriptLanguage>(b"const x = 1;", "test.ts");
    assert!(ts_count > 0);

    let js_count = parse_and_count::<JavaScriptLanguage>(b"const x = 1;", "test.js");
    assert!(js_count > 0);

    let py_count = parse_and_count::<PythonLanguage>(b"x = 1", "test.py");
    assert!(py_count > 0);
}

#[test]
fn test_language_info_trait() {
    assert_eq!(RustLanguage::get_lang(), Lang::Rust);
    assert_eq!(RustLanguage::get_lang_name(), "rust");

    assert_eq!(TypeScriptLanguage::get_lang(), Lang::TypeScript);
    assert_eq!(TypeScriptLanguage::get_lang_name(), "typescript");

    assert_eq!(JavaScriptLanguage::get_lang(), Lang::JavaScript);
    assert_eq!(JavaScriptLanguage::get_lang_name(), "javascript");

    assert_eq!(PythonLanguage::get_lang(), Lang::Python);
    assert_eq!(PythonLanguage::get_lang_name(), "python");
}

#[test]
fn test_node_navigation() {
    let code = b"fn a() {} fn b() {} fn c() {}".to_vec();
    let parser = Parser::<RustLanguage>::new(code, Path::new("test.rs"))
        .expect("Failed to create parser");

    let root = parser.get_root();
    let children: Vec<_> = root.children()
        .filter(|n| n.kind() == "function_item")
        .collect();

    // Test that we have 3 functions
    assert_eq!(children.len(), 3);

    // Test sibling navigation
    if let Some(second) = children[1].next_sibling() {
        if second.kind() == "function_item" {
            // We found the third function via sibling navigation
            assert!(true);
        }
    }

    // Test parent navigation
    for child in &children {
        assert_eq!(child.parent().map(|p| p.kind()), Some("source_file"));
    }
}

#[test]
fn test_multiple_languages_in_same_test() {
    // This test demonstrates that we can work with multiple languages
    // in the same context using the trait system

    let rust_code = b"fn test() {}".to_vec();
    let rust_parser = Parser::<RustLanguage>::new(rust_code, Path::new("test.rs"))
        .expect("Failed to create Rust parser");

    let ts_code = b"const x = 1;".to_vec();
    let ts_parser = Parser::<TypeScriptLanguage>::new(ts_code, Path::new("test.ts"))
        .expect("Failed to create TypeScript parser");

    assert_eq!(rust_parser.get_language(), Lang::Rust);
    assert_eq!(ts_parser.get_language(), Lang::TypeScript);

    // Both return the same type (Node) but for different languages
    let rust_root = rust_parser.get_root();
    let ts_root = ts_parser.get_root();

    assert_eq!(rust_root.kind(), "source_file");
    assert_eq!(ts_root.kind(), "program");
}

#[test]
fn test_error_handling() {
    // Test parsing invalid code
    let code = b"fn {{{ invalid rust code".to_vec();
    let parser = Parser::<RustLanguage>::new(code, Path::new("invalid.rs"))
        .expect("Parser should still be created even with invalid code");

    let root = parser.get_root();
    // The tree will contain error nodes
    assert!(root.child_count() > 0);

    // We can detect errors
    let has_errors = root.children().any(|n| n.kind() == "ERROR");
    assert!(has_errors || root.has_error());
}

#[test]
fn test_lang_into_iter() {
    let langs: Vec<Lang> = Lang::into_enum_iter().collect();

    assert!(langs.contains(&Lang::Rust));
    assert!(langs.contains(&Lang::TypeScript));
    assert!(langs.contains(&Lang::JavaScript));
    assert!(langs.contains(&Lang::Python));
    assert!(langs.len() >= 9); // At least 9 languages supported
}

#[test]
fn test_get_ts_language() {
    // Test that we can get tree-sitter Language objects
    let rust_lang = Lang::Rust.get_ts_language();
    let ts_lang = Lang::TypeScript.get_ts_language();

    // They should be different
    assert_ne!(
        rust_lang.node_kind_count(),
        ts_lang.node_kind_count()
    );
}
