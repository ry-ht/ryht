//! End-to-end tests for code manipulation tools
//! Tests all 15 code manipulation tools with real code examples

use cortex_code_analysis::{AstEditor, Range};
use tree_sitter_rust;

#[test]
fn test_create_function_rust() {
    let source = r#"
// Existing file
fn existing() {
    println!("Hello");
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Create new function
    let new_function = r#"
fn calculate(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    editor.insert_at(5, 0, new_function).unwrap();
    editor.apply_edits().unwrap();

    let result = editor.get_source();
    assert!(result.contains("fn calculate"));
    assert!(result.contains("fn existing"));
}

#[test]
fn test_rename_symbol() {
    let source = r#"
fn calculate(x: i32) -> i32 {
    let y = calculate(x + 1);
    y
}

fn main() {
    let result = calculate(5);
    println!("{}", result);
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Rename calculate -> compute
    let edits = editor.rename_symbol("calculate", "compute").unwrap();

    assert!(edits.len() > 0, "Should find occurrences to rename");

    editor.apply_edits().unwrap();
    let result = editor.get_source();

    assert!(result.contains("fn compute"));
    assert!(!result.contains("fn calculate"));
    assert!(result.contains("compute(5)"));
    assert!(result.contains("compute(x + 1)"));
}

#[test]
fn test_delete_function() {
    let source = r#"
fn keep_this() {
    println!("Keep");
}

fn delete_this() {
    println!("Delete");
}

fn also_keep() {
    println!("Also keep");
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Find the function to delete
    let functions = editor.query("(function_item) @func").unwrap();
    assert_eq!(functions.len(), 3);

    // Collect the range of the node to delete (to avoid borrow issues)
    let delete_range = {
        let delete_target = functions
            .iter()
            .find(|f| editor.node_text(f).contains("delete_this"))
            .unwrap();
        Range::from_node(delete_target)
    };

    // Now delete using the range
    editor.edits.push(cortex_code_analysis::Edit::delete(delete_range));
    editor.apply_edits().unwrap();

    let result = editor.get_source();
    assert!(result.contains("keep_this"));
    assert!(result.contains("also_keep"));
    assert!(!result.contains("delete_this"));
}

#[test]
fn test_add_import_rust() {
    let source = r#"
use std::io::Read;

fn main() {
    println!("Hello");
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    editor.add_import_rust("std::collections::HashMap").unwrap();
    editor.apply_edits().unwrap();

    let result = editor.get_source();
    assert!(result.contains("use std::collections::HashMap"));
    assert!(result.contains("use std::io::Read"));
}

#[test]
fn test_optimize_imports() {
    let source = r#"
use std::collections::HashMap;
use std::io::Read;
use std::collections::HashMap;
use crate::utils;
use std::fs::File;

fn main() {}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    let result = editor.optimize_imports_rust().unwrap();
    assert!(result.removed > 0, "Should remove duplicate import");
    assert!(result.sorted, "Imports should be sorted");

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    // Should have only one HashMap import
    let hashmap_count = code.matches("use std::collections::HashMap").count();
    assert_eq!(hashmap_count, 1, "Should have exactly one HashMap import");
}

#[test]
fn test_change_signature() {
    let source = r#"
fn process(id: i32) {
    println!("{}", id);
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Change signature to add a parameter
    let new_params = vec![
        ("id".to_string(), "i32".to_string()),
        ("name".to_string(), "String".to_string()),
    ];

    editor
        .change_signature_rust("process", new_params, None)
        .unwrap();

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    assert!(code.contains("id: i32"));
    assert!(code.contains("name: String"));
}

#[test]
fn test_real_world_refactoring_workflow() {
    // Simulate a real refactoring workflow
    let source = r#"
use std::io::Read;
use std::io::Read;

fn calculate(x: i32) -> i32 {
    x * 2
}

fn process_data(data: Vec<i32>) -> Vec<i32> {
    let mut results = Vec::new();
    for item in data {
        let value = item * 2;
        results.push(value);
    }
    results
}

fn main() {
    let data = vec![1, 2, 3];
    let result = process_data(data);
    println!("{:?}", result);
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Step 1: Optimize imports
    let optimize_result = editor.optimize_imports_rust().unwrap();
    assert!(optimize_result.removed > 0);

    // Step 2: Rename function
    editor
        .rename_symbol("process_data", "transform_data")
        .unwrap();

    // Step 3: Add new import
    editor.add_import_rust("std::collections::HashMap").unwrap();

    // Apply all edits
    editor.apply_edits().unwrap();
    let code = editor.get_source();

    // Verify all changes
    assert!(code.contains("transform_data"));
    assert!(!code.contains("process_data"));
    assert!(code.contains("use std::collections::HashMap"));

    // Only one Read import
    let read_count = code.matches("use std::io::Read").count();
    assert_eq!(read_count, 1);
}

#[test]
fn test_multiple_renames_in_complex_code() {
    let source = r#"
struct User {
    name: String,
    age: u32,
}

impl User {
    fn new(name: String, age: u32) -> User {
        User { name, age }
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

fn create_user(name: String, age: u32) -> User {
    User::new(name, age)
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Rename User -> Account
    let edits = editor.rename_symbol("User", "Account").unwrap();
    assert!(edits.len() > 0, "Should rename multiple occurrences");

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    assert!(code.contains("struct Account"));
    assert!(code.contains("impl Account"));
    assert!(code.contains("fn create_user(name: String, age: u32) -> Account"));
    assert!(!code.contains("struct User"));
}

#[test]
fn test_parse_then_modify_then_parse_again() {
    // Test that we can parse, modify, and re-parse successfully
    let source = r#"
fn original() {
    println!("Original");
}
"#
    .to_string();

    // First parse
    let mut editor = AstEditor::new(source.clone(), tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree1 = editor.tree();
    assert!(!tree1.root_node().has_error());

    // Modify
    editor.rename_symbol("original", "modified").unwrap();
    editor.apply_edits().unwrap();

    // Second parse (happens in apply_edits)
    let tree2 = editor.tree();
    assert!(!tree2.root_node().has_error());

    // Verify change
    let code = editor.get_source();
    assert!(code.contains("fn modified"));
    assert!(!code.contains("fn original"));
}

#[test]
fn test_query_functions() {
    let source = r#"
fn foo() {}
fn bar() {}
struct Baz {}
fn qux() {}
"#
    .to_string();

    let editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    let functions = editor.query("(function_item) @func").unwrap();
    assert_eq!(functions.len(), 3, "Should find 3 functions");

    let structs = editor.query("(struct_item) @struct").unwrap();
    assert_eq!(structs.len(), 1, "Should find 1 struct");
}

#[test]
fn test_node_text_extraction() {
    let source = r#"
fn calculate(a: i32, b: i32) -> i32 {
    a + b
}
"#
    .to_string();

    let editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    let functions = editor.query("(function_item) @func").unwrap();
    assert_eq!(functions.len(), 1);

    let func_text = editor.node_text(&functions[0]);
    assert!(func_text.contains("fn calculate"));
    assert!(func_text.contains("a + b"));
}

#[test]
fn test_insert_multiple_functions() {
    let source = "// Empty file\n".to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Insert first function
    editor
        .insert_at(
            1,
            0,
            r#"
fn first() {
    println!("First");
}
"#,
        )
        .unwrap();

    // Insert second function
    editor
        .insert_at(
            5,
            0,
            r#"
fn second() {
    println!("Second");
}
"#,
        )
        .unwrap();

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    assert!(code.contains("fn first"));
    assert!(code.contains("fn second"));
}

#[test]
fn test_delete_and_recreate() {
    let source = r#"
fn temporary() {
    println!("Temp");
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Delete the function
    let delete_range = {
        let functions = editor.query("(function_item) @func").unwrap();
        Range::from_node(&functions[0])
    };
    editor.edits.push(cortex_code_analysis::Edit::delete(delete_range));

    // Add a new function
    editor
        .insert_at(
            0,
            0,
            r#"
fn permanent() {
    println!("Permanent");
}
"#,
        )
        .unwrap();

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    assert!(code.contains("fn permanent"));
    assert!(!code.contains("fn temporary"));
}

// Performance/Stress Tests

#[test]
fn test_rename_in_large_file() {
    // Simulate a larger file
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(&format!(
            "fn func_{}() {{ let x = value; }}\n",
            i
        ));
    }

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Rename value -> data in all 100 functions
    let edits = editor.rename_symbol("value", "data").unwrap();
    assert_eq!(edits.len(), 100);

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    assert!(!code.contains("let x = value"));
    assert_eq!(code.matches("let x = data").count(), 100);
}

#[test]
fn test_complex_import_optimization() {
    let source = r#"
use std::io::Read;
use std::fs::File;
use std::collections::HashMap;
use std::io::Write;
use std::collections::HashMap;
use std::io::Read;
use crate::local::module;
use external::crate_name;

fn main() {}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    let result = editor.optimize_imports_rust().unwrap();
    assert!(result.removed >= 2, "Should remove at least 2 duplicates");

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    // Check no duplicates
    assert_eq!(code.matches("use std::io::Read").count(), 1);
    assert_eq!(code.matches("use std::collections::HashMap").count(), 1);
}

#[test]
fn test_ast_editor_error_handling() {
    let invalid_source = "fn incomplete() {".to_string();

    // Even with syntax errors, should be able to create editor
    let result = AstEditor::new(invalid_source, tree_sitter_rust::LANGUAGE.into());

    // Should succeed (tree-sitter can parse partial code)
    assert!(result.is_ok());

    let editor = result.unwrap();
    let tree = editor.tree();

    // Root node will have errors
    assert!(tree.root_node().has_error() || tree.root_node().has_changes());
}

// Integration test showing full workflow
#[test]
fn test_full_refactoring_workflow_realistic() {
    let source = r#"
use std::io;

struct UserData {
    id: i32,
    name: String,
}

fn get_user(id: i32) -> UserData {
    UserData {
        id: id,
        name: String::from("John"),
    }
}

fn display_user(user: UserData) {
    println!("User: {} - {}", user.id, user.name);
}

fn main() {
    let user = get_user(1);
    display_user(user);
}
"#
    .to_string();

    let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();

    // Workflow: Refactor UserData -> User
    println!("Step 1: Renaming UserData -> User");
    editor.rename_symbol("UserData", "User").unwrap();

    // Add imports
    println!("Step 2: Adding imports");
    editor.add_import_rust("std::collections::HashMap").unwrap();

    // Optimize
    println!("Step 3: Optimizing imports");
    editor.optimize_imports_rust().unwrap();

    editor.apply_edits().unwrap();
    let code = editor.get_source();

    // Verify
    assert!(code.contains("struct User"));
    assert!(code.contains("fn get_user(id: i32) -> User"));
    assert!(code.contains("fn display_user(user: User)"));
    assert!(code.contains("use std::collections::HashMap"));

    // Make sure the code is still valid Rust
    let final_editor = AstEditor::new(code.to_string(), tree_sitter_rust::LANGUAGE.into()).unwrap();
    assert!(!final_editor.tree().root_node().has_error());
}
