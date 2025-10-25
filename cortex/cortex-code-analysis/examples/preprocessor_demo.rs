//! Example demonstrating C/C++ preprocessor directive extraction and analysis.
//!
//! Run with: cargo run --example preprocessor_demo

use cortex_code_analysis::{
    TreeSitterWrapper,
    preprocessor::{extract_preprocessor, build_include_graph, get_all_macros, PreprocResults},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    println!("=== C/C++ Preprocessor Analysis Demo ===\n");

    // Example 1: Basic include and macro extraction
    println!("Example 1: Basic Extraction");
    println!("----------------------------");

    let source1 = r#"
#include <stdio.h>
#include <stdlib.h>
#include "myheader.h"
#define MAX_BUFFER_SIZE 1024
#define MIN_BUFFER_SIZE 64
#define ARRAY_LENGTH(arr) (sizeof(arr) / sizeof((arr)[0]))
    "#;

    let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into())?;
    let tree = parser.parse(source1)?;

    let mut results = PreprocResults::default();
    extract_preprocessor(&tree, source1, Path::new("example1.cpp"), &mut results)?;

    let file_data = results.files.get(Path::new("example1.cpp")).unwrap();
    println!("Direct includes: {:?}", file_data.direct_includes);
    println!("Macros defined: {:?}", file_data.macros);
    println!();

    // Example 2: Complex include hierarchy
    println!("Example 2: Include Dependency Graph");
    println!("------------------------------------");

    // Simulate multiple files with includes
    let file_a = r#"
#include "file_b.h"
#include "file_c.h"
#define FILE_A_MACRO 1
    "#;

    let file_b = r#"
#include "file_c.h"
#define FILE_B_MACRO 2
    "#;

    let file_c = r#"
#define FILE_C_MACRO 3
    "#;

    let mut results2 = PreprocResults::default();

    // Parse all files
    let tree_a = parser.parse(file_a)?;
    extract_preprocessor(&tree_a, file_a, Path::new("/project/file_a.h"), &mut results2)?;

    let tree_b = parser.parse(file_b)?;
    extract_preprocessor(&tree_b, file_b, Path::new("/project/file_b.h"), &mut results2)?;

    let tree_c = parser.parse(file_c)?;
    extract_preprocessor(&tree_c, file_c, Path::new("/project/file_c.h"), &mut results2)?;

    // Build mapping of filenames to full paths (for include resolution)
    let mut all_files: HashMap<String, Vec<PathBuf>> = HashMap::new();
    all_files.insert("file_b.h".to_string(), vec![PathBuf::from("/project/file_b.h")]);
    all_files.insert("file_c.h".to_string(), vec![PathBuf::from("/project/file_c.h")]);

    // Build the dependency graph
    build_include_graph(&mut results2.files, &all_files);

    // Show results
    for (path, data) in &results2.files {
        println!("File: {:?}", path);
        println!("  Direct includes: {:?}", data.direct_includes);
        println!("  Indirect includes: {:?}", data.indirect_includes);
        println!("  Macros: {:?}", data.macros);
        println!();
    }

    // Example 3: Get all macros visible to a file
    println!("Example 3: All Visible Macros");
    println!("------------------------------");

    let all_macros = get_all_macros(Path::new("/project/file_a.h"), &results2.files);
    println!("All macros visible to file_a.h: {:?}", all_macros);
    println!();

    // Example 4: Special keywords filtering
    println!("Example 4: Keyword Filtering");
    println!("----------------------------");

    let source4 = r#"
#define NULL 0
#define size_t unsigned long
#define MY_CUSTOM_MACRO 42
#define bool int
    "#;

    let tree4 = parser.parse(source4)?;
    let mut results4 = PreprocResults::default();
    extract_preprocessor(&tree4, source4, Path::new("keywords.cpp"), &mut results4)?;

    let file_data4 = results4.files.get(Path::new("keywords.cpp")).unwrap();
    println!("Macros (special keywords filtered out): {:?}", file_data4.macros);
    println!("Notice that NULL, size_t, and bool are excluded as they are standard C/C++ keywords.");
    println!();

    // Example 5: Function-like macros
    println!("Example 5: Function-like Macros");
    println!("--------------------------------");

    let source5 = r#"
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define SQUARE(x) ((x) * (x))
    "#;

    let tree5 = parser.parse(source5)?;
    let mut results5 = PreprocResults::default();
    extract_preprocessor(&tree5, source5, Path::new("macros.cpp"), &mut results5)?;

    let file_data5 = results5.files.get(Path::new("macros.cpp")).unwrap();
    println!("Function-like macros: {:?}", file_data5.macros);
    println!();

    println!("=== Demo Complete ===");

    Ok(())
}
