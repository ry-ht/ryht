//! Integration tests for C/C++ preprocessor and macro handling
//!
//! This test file demonstrates the complete workflow of:
//! 1. Extracting preprocessor directives from C/C++ code
//! 2. Building dependency graphs from includes
//! 3. Replacing macros for improved parsing
//! 4. Using special keyword and predefined macro detection

use cortex_code_analysis::{
    TreeSitterWrapper,
    PreprocResults, PreprocFile,
    extract_preprocessor, build_include_graph, get_all_macros,
    replace_macros, prepare_file, is_predefined_macro,
    is_special_keyword, get_all_special_keywords, get_all_predefined_macros,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[test]
fn test_preprocessor_extraction_integration() {
    let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include "myheader.h"

#define MAX_SIZE 1024
#define MIN_SIZE 64
#define BUFFER_SIZE (MAX_SIZE * 2)
#define DEBUG_MODE

#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(x, y) ((x) < (y) ? (x) : (y))

// This should not be included (it's a comment)
// #define COMMENTED_MACRO 42

void test() {
    printf("Hello, World!\n");
}
"#;

    let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source).unwrap();
    let mut results = PreprocResults::default();

    extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

    let file_data = results.files.get(Path::new("test.cpp")).unwrap();

    // Check includes
    assert_eq!(file_data.direct_includes.len(), 3);
    assert!(file_data.direct_includes.contains("stdio.h"));
    assert!(file_data.direct_includes.contains("stdlib.h"));
    assert!(file_data.direct_includes.contains("myheader.h"));

    // Check macros (6 macros total: MAX_SIZE, MIN_SIZE, BUFFER_SIZE, DEBUG_MODE, MAX, MIN)
    assert_eq!(file_data.macros.len(), 6);
    assert!(file_data.macros.contains("MAX_SIZE"));
    assert!(file_data.macros.contains("MIN_SIZE"));
    assert!(file_data.macros.contains("BUFFER_SIZE"));
    assert!(file_data.macros.contains("DEBUG_MODE"));
    assert!(file_data.macros.contains("MAX"));
    assert!(file_data.macros.contains("MIN"));

    // Commented macro should not be included
    assert!(!file_data.macros.contains("COMMENTED_MACRO"));
}

#[test]
fn test_macro_replacement_integration() {
    let mut macros = HashSet::new();
    macros.insert("MAX_SIZE".to_string());
    macros.insert("MIN_SIZE".to_string());
    macros.insert("BUFFER_SIZE".to_string());

    let code = b"int buffer[MAX_SIZE]; int min = MIN_SIZE; int total = BUFFER_SIZE;";
    let result = replace_macros(code, &macros);

    assert!(result.is_some());
    let replaced = result.unwrap();

    // All macros should be replaced with $ characters
    assert!(replaced.contains(&b'$'));

    // Length should be preserved
    assert_eq!(code.len(), replaced.len());

    // Original identifiers should not be in replaced code
    let replaced_str = String::from_utf8(replaced).unwrap();
    assert!(!replaced_str.contains("MAX_SIZE"));
    assert!(!replaced_str.contains("MIN_SIZE"));
    assert!(!replaced_str.contains("BUFFER_SIZE"));

    // Other identifiers should be preserved
    assert!(replaced_str.contains("buffer"));
    assert!(replaced_str.contains("min"));
    assert!(replaced_str.contains("total"));
}

#[test]
fn test_prepare_file_integration() {
    let mut macros = HashSet::new();
    macros.insert("CUSTOM_MACRO".to_string());

    // Code without macros
    let code1 = b"int main() { return 0; }";
    let result1 = prepare_file(code1, &macros);
    assert_eq!(result1, code1.to_vec());

    // Code with macros
    let code2 = b"int x = CUSTOM_MACRO;";
    let result2 = prepare_file(code2, &macros);
    assert_ne!(result2, code2.to_vec());
    assert_eq!(result2.len(), code2.len());
}

#[test]
fn test_special_keywords_integration() {
    // Test basic types
    assert!(is_special_keyword("int"));
    assert!(is_special_keyword("char"));
    assert!(is_special_keyword("double"));
    assert!(is_special_keyword("void"));

    // Test fixed-width types
    assert!(is_special_keyword("int32_t"));
    assert!(is_special_keyword("uint64_t"));

    // Test standard library types
    assert!(is_special_keyword("size_t"));
    assert!(is_special_keyword("NULL"));

    // Test language keywords
    assert!(is_special_keyword("const"));
    assert!(is_special_keyword("static"));
    assert!(is_special_keyword("constexpr"));

    // Test user-defined identifiers
    assert!(!is_special_keyword("MY_MACRO"));
    assert!(!is_special_keyword("custom_type"));

    // Verify we have a good collection
    let keywords = get_all_special_keywords();
    assert!(keywords.len() > 50);
    assert!(keywords.contains(&"int"));
    assert!(keywords.contains(&"NULL"));
}

#[test]
fn test_predefined_macros_integration() {
    // Test integer limits
    assert!(is_predefined_macro("INT32_MAX"));
    assert!(is_predefined_macro("UINT64_MIN"));

    // Test printf format specifiers
    assert!(is_predefined_macro("PRId32"));
    assert!(is_predefined_macro("PRIu64"));
    assert!(is_predefined_macro("PRIxMAX"));

    // Test scanf format specifiers
    assert!(is_predefined_macro("SCNd32"));
    assert!(is_predefined_macro("SCNu64"));

    // Test user-defined macros
    assert!(!is_predefined_macro("MY_CUSTOM_MACRO"));
    assert!(!is_predefined_macro("BUFFER_SIZE"));

    // Verify we have a comprehensive collection
    let macros = get_all_predefined_macros();
    assert!(macros.len() > 200);
    assert!(macros.contains(&"INT32_MAX"));
    assert!(macros.contains(&"PRId64"));
}

#[test]
fn test_include_graph_simple() {
    let mut files = HashMap::new();
    let mut all_files = HashMap::new();

    // Create main.c that includes util.h
    let mut main_file = PreprocFile::default();
    main_file.direct_includes.insert("util.h".to_string());
    files.insert(PathBuf::from("/project/src/main.c"), main_file);

    // Create util.h
    let util_file = PreprocFile::default();
    files.insert(PathBuf::from("/project/include/util.h"), util_file);

    // Map filenames to paths
    all_files.insert(
        "util.h".to_string(),
        vec![PathBuf::from("/project/include/util.h")],
    );

    build_include_graph(&mut files, &all_files);

    // Check that main.c now knows about util.h through indirect includes
    let main_data = files.get(&PathBuf::from("/project/src/main.c")).unwrap();
    assert_eq!(main_data.indirect_includes.len(), 1);
    assert!(main_data.indirect_includes.contains("/project/include/util.h"));
}

#[test]
fn test_include_graph_transitive() {
    let mut files = HashMap::new();
    let mut all_files = HashMap::new();

    // Create chain: main.c -> util.h -> config.h
    let mut main_file = PreprocFile::default();
    main_file.direct_includes.insert("util.h".to_string());
    files.insert(PathBuf::from("/project/main.c"), main_file);

    let mut util_file = PreprocFile::default();
    util_file.direct_includes.insert("config.h".to_string());
    files.insert(PathBuf::from("/project/util.h"), util_file);

    let config_file = PreprocFile::default();
    files.insert(PathBuf::from("/project/config.h"), config_file);

    all_files.insert(
        "util.h".to_string(),
        vec![PathBuf::from("/project/util.h")],
    );
    all_files.insert(
        "config.h".to_string(),
        vec![PathBuf::from("/project/config.h")],
    );

    build_include_graph(&mut files, &all_files);

    // main.c should see both util.h and config.h
    let main_data = files.get(&PathBuf::from("/project/main.c")).unwrap();
    assert_eq!(main_data.indirect_includes.len(), 2);
    assert!(main_data.indirect_includes.contains("/project/util.h"));
    assert!(main_data.indirect_includes.contains("/project/config.h"));
}

#[test]
fn test_get_all_macros_integration() {
    let mut files = HashMap::new();

    // File 1 has MACRO1 and MACRO2
    let mut file1 = PreprocFile::default();
    file1.macros.insert("MACRO1".to_string());
    file1.macros.insert("MACRO2".to_string());

    // File 2 has MACRO3
    let mut file2 = PreprocFile::default();
    file2.macros.insert("MACRO3".to_string());

    // File 3 has MACRO4
    let mut file3 = PreprocFile::default();
    file3.macros.insert("MACRO4".to_string());

    files.insert(PathBuf::from("file1.h"), file1);
    files.insert(PathBuf::from("file2.h"), file2);
    files.insert(PathBuf::from("file3.h"), file3);

    // file1 indirectly includes file2 and file3
    files.get_mut(&PathBuf::from("file1.h")).unwrap()
        .indirect_includes.insert("file2.h".to_string());
    files.get_mut(&PathBuf::from("file1.h")).unwrap()
        .indirect_includes.insert("file3.h".to_string());

    // Get all macros visible to file1
    let all_macros = get_all_macros(Path::new("file1.h"), &files);

    // Should see all 4 macros
    assert_eq!(all_macros.len(), 4);
    assert!(all_macros.contains("MACRO1"));
    assert!(all_macros.contains("MACRO2"));
    assert!(all_macros.contains("MACRO3"));
    assert!(all_macros.contains("MACRO4"));

    // file2 should only see its own macro
    let file2_macros = get_all_macros(Path::new("file2.h"), &files);
    assert_eq!(file2_macros.len(), 1);
    assert!(file2_macros.contains("MACRO3"));
}

#[test]
fn test_special_keywords_not_counted_as_macros() {
    let source = r#"
#define NULL 0
#define MY_MACRO 42
#define size_t unsigned long
#define int32_t int
#define CUSTOM_TYPE 1
#define bool int
    "#;

    let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source).unwrap();
    let mut results = PreprocResults::default();

    extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

    let file_data = results.files.get(Path::new("test.cpp")).unwrap();

    // Only MY_MACRO and CUSTOM_TYPE should be counted
    // NULL, size_t, int32_t, and bool are special keywords
    assert_eq!(file_data.macros.len(), 2);
    assert!(file_data.macros.contains("MY_MACRO"));
    assert!(file_data.macros.contains("CUSTOM_TYPE"));
    assert!(!file_data.macros.contains("NULL"));
    assert!(!file_data.macros.contains("size_t"));
    assert!(!file_data.macros.contains("int32_t"));
    assert!(!file_data.macros.contains("bool"));
}

#[test]
fn test_end_to_end_workflow() {
    // This test demonstrates the complete workflow:
    // 1. Extract preprocessor directives
    // 2. Get all visible macros
    // 3. Replace macros in code
    // 4. Parse the cleaned code

    let header_source = r#"
#define MAX_BUFFER 4096
#define MIN_BUFFER 128
    "#;

    let main_source = r#"
#include "myheader.h"

int buffer[MAX_BUFFER];
int small[MIN_BUFFER];

void process() {
    // ...
}
    "#;

    // Step 1: Extract from header
    let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
    let tree = parser.parse(header_source).unwrap();
    let mut results = PreprocResults::default();
    extract_preprocessor(&tree, header_source, Path::new("myheader.h"), &mut results).unwrap();

    // Step 2: Extract from main
    let tree = parser.parse(main_source).unwrap();
    extract_preprocessor(&tree, main_source, Path::new("main.c"), &mut results).unwrap();

    // Step 3: Build include graph
    let mut all_files = HashMap::new();
    all_files.insert("myheader.h".to_string(), vec![PathBuf::from("myheader.h")]);
    build_include_graph(&mut results.files, &all_files);

    // Step 4: Get all macros visible to main.c
    let macros = get_all_macros(Path::new("main.c"), &results.files);

    // Should see macros from header
    assert!(macros.contains("MAX_BUFFER"));
    assert!(macros.contains("MIN_BUFFER"));

    // Step 5: Replace macros in main.c source
    let prepared_code = prepare_file(main_source.as_bytes(), &macros);

    // Macros should be replaced
    let prepared_str = String::from_utf8(prepared_code.clone()).unwrap();
    assert!(!prepared_str.contains("MAX_BUFFER"));
    assert!(!prepared_str.contains("MIN_BUFFER"));
    assert!(prepared_str.contains("buffer"));
    assert!(prepared_str.contains("small"));

    // Step 6: Parse the prepared code
    let tree = parser.parse(&String::from_utf8(prepared_code).unwrap()).unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_comprehensive_macro_coverage() {
    // Test that our system handles various macro patterns

    let source = r#"
// Object-like macros
#define SIMPLE 42
#define COMPLEX (1 << 16)
#define STRING_MACRO "hello"

// Function-like macros
#define ADD(x, y) ((x) + (y))
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define PRINT(msg) printf("%s\n", msg)

// Multi-line macros
#define MULTI_LINE \
    do { \
        int x = 1; \
        int y = 2; \
    } while(0)

// Macros with standard types (should be excluded)
#define MY_NULL NULL
#define MY_SIZE size_t
    "#;

    let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source).unwrap();
    let mut results = PreprocResults::default();

    extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

    let file_data = results.files.get(Path::new("test.cpp")).unwrap();

    // Should find all user-defined macros
    assert!(file_data.macros.contains("SIMPLE"));
    assert!(file_data.macros.contains("COMPLEX"));
    assert!(file_data.macros.contains("STRING_MACRO"));
    assert!(file_data.macros.contains("ADD"));
    assert!(file_data.macros.contains("MAX"));
    assert!(file_data.macros.contains("PRINT"));
    assert!(file_data.macros.contains("MULTI_LINE"));
    assert!(file_data.macros.contains("MY_NULL"));
    assert!(file_data.macros.contains("MY_SIZE"));

    // Total count
    assert_eq!(file_data.macros.len(), 9);
}
