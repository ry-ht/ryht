//! Code Manipulation Performance Benchmarks
//!
//! Comprehensive benchmarks for:
//! - Parsing (100 LOC <10ms, 1K LOC <50ms, 10K LOC <500ms)
//! - AST editing (add function <20ms, rename <50ms, extract method <100ms)
//! - Code generation (100 LOC <10ms, 1K LOC <100ms)
//! - Dependency extraction

use cortex_parser::{
    parser::{Parser, Language},
    ast_editor::{AstEditor, EditOperation},
    dependency_extractor::DependencyExtractor,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;

// ==============================================================================
// Test Code Generation Helpers
// ==============================================================================

fn generate_rust_code(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("// Auto-generated Rust code for benchmarking\n\n");
    code.push_str("use std::collections::HashMap;\n");
    code.push_str("use std::sync::Arc;\n\n");

    let funcs_needed = lines / 4; // Each function is ~4 lines

    for i in 0..funcs_needed {
        code.push_str(&format!(
            "pub fn function_{}(x: i32, y: i32) -> i32 {{\n\
             \tlet result = x + y + {};\n\
             \tresult * 2\n\
             }}\n\n",
            i, i
        ));
    }

    code
}

fn generate_typescript_code(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("// Auto-generated TypeScript code for benchmarking\n\n");
    code.push_str("import { Component } from 'react';\n\n");

    let funcs_needed = lines / 4;

    for i in 0..funcs_needed {
        code.push_str(&format!(
            "export function function{}(x: number, y: number): number {{\n\
             \tconst result = x + y + {};\n\
             \treturn result * 2;\n\
             }}\n\n",
            i, i
        ));
    }

    code
}

fn generate_complex_rust_module(lines: usize) -> String {
    let mut code = String::new();
    code.push_str(
        "//! Complex Rust module with various constructs\n\n\
         use std::collections::{HashMap, HashSet, BTreeMap};\n\
         use std::sync::{Arc, Mutex, RwLock};\n\
         use std::error::Error;\n\n"
    );

    // Add structs
    for i in 0..(lines / 20) {
        code.push_str(&format!(
            "#[derive(Debug, Clone)]\n\
             pub struct DataStruct{} {{\n\
             \tpub id: u64,\n\
             \tpub name: String,\n\
             \tpub data: Vec<u8>,\n\
             \tpub metadata: HashMap<String, String>,\n\
             }}\n\n",
            i
        ));
    }

    // Add trait
    code.push_str(
        "pub trait DataProcessor {\n\
         \tfn process(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>>;\n\
         \tfn validate(&self) -> bool;\n\
         }\n\n"
    );

    // Add impl blocks
    for i in 0..(lines / 30) {
        code.push_str(&format!(
            "impl DataStruct{} {{\n\
             \tpub fn new(id: u64, name: String) -> Self {{\n\
             \t\tSelf {{\n\
             \t\t\tid,\n\
             \t\t\tname,\n\
             \t\t\tdata: Vec::new(),\n\
             \t\t\tmetadata: HashMap::new(),\n\
             \t\t}}\n\
             \t}}\n\
             \n\
             \tpub fn add_data(&mut self, data: Vec<u8>) {{\n\
             \t\tself.data.extend(data);\n\
             \t}}\n\
             \n\
             \tpub fn get_metadata(&self, key: &str) -> Option<&String> {{\n\
             \t\tself.metadata.get(key)\n\
             \t}}\n\
             }}\n\n",
            i
        ));
    }

    code
}

// ==============================================================================
// Parsing Performance Benchmarks
// ==============================================================================

fn bench_parsing_rust(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_rust");
    group.significance_level(0.05).sample_size(100);

    // Parse 100 LOC - Target: <10ms
    let code_100 = generate_rust_code(100);
    group.throughput(Throughput::Elements(100));
    group.bench_function("parse_100_loc", |b| {
        b.iter(|| {
            let parser = Parser::new(Language::Rust);
            let ast = parser.parse(&code_100).unwrap();
            black_box(ast);
        });
    });

    // Parse 1K LOC - Target: <50ms
    let code_1k = generate_rust_code(1000);
    group.throughput(Throughput::Elements(1000));
    group.bench_function("parse_1000_loc", |b| {
        b.iter(|| {
            let parser = Parser::new(Language::Rust);
            let ast = parser.parse(&code_1k).unwrap();
            black_box(ast);
        });
    });

    // Parse 10K LOC - Target: <500ms
    let code_10k = generate_rust_code(10_000);
    group.throughput(Throughput::Elements(10_000));
    group.measurement_time(Duration::from_secs(15));
    group.bench_function("parse_10000_loc", |b| {
        b.iter(|| {
            let parser = Parser::new(Language::Rust);
            let ast = parser.parse(&code_10k).unwrap();
            black_box(ast);
        });
    });

    // Parse complex module with structs, traits, impls
    let complex_code = generate_complex_rust_module(1000);
    group.throughput(Throughput::Elements(1000));
    group.bench_function("parse_complex_1000_loc", |b| {
        b.iter(|| {
            let parser = Parser::new(Language::Rust);
            let ast = parser.parse(&complex_code).unwrap();
            black_box(ast);
        });
    });

    group.finish();
}

fn bench_parsing_typescript(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_typescript");
    group.significance_level(0.05).sample_size(100);

    // Parse 100 LOC TypeScript
    let code_100 = generate_typescript_code(100);
    group.throughput(Throughput::Elements(100));
    group.bench_function("parse_100_loc", |b| {
        b.iter(|| {
            let parser = Parser::new(Language::TypeScript);
            let ast = parser.parse(&code_100).unwrap();
            black_box(ast);
        });
    });

    // Parse 1K LOC TypeScript
    let code_1k = generate_typescript_code(1000);
    group.throughput(Throughput::Elements(1000));
    group.bench_function("parse_1000_loc", |b| {
        b.iter(|| {
            let parser = Parser::new(Language::TypeScript);
            let ast = parser.parse(&code_1k).unwrap();
            black_box(ast);
        });
    });

    group.finish();
}

// ==============================================================================
// AST Query Benchmarks
// ==============================================================================

fn bench_ast_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_queries");
    group.significance_level(0.05).sample_size(100);

    let code = generate_complex_rust_module(1000);
    let parser = Parser::new(Language::Rust);
    let ast = parser.parse(&code).unwrap();

    // Find all functions - Target: <10ms
    group.bench_function("find_all_functions", |b| {
        b.iter(|| {
            let functions = parser.find_functions(&ast);
            black_box(functions);
        });
    });

    // Find all structs - Target: <10ms
    group.bench_function("find_all_structs", |b| {
        b.iter(|| {
            let structs = parser.find_structs(&ast);
            black_box(structs);
        });
    });

    // Find all imports - Target: <5ms
    group.bench_function("find_all_imports", |b| {
        b.iter(|| {
            let imports = parser.find_imports(&ast);
            black_box(imports);
        });
    });

    // Find node by position - Target: <5ms
    group.bench_function("find_node_at_position", |b| {
        b.iter(|| {
            let node = parser.find_node_at_line(&ast, 50);
            black_box(node);
        });
    });

    // Get function signature - Target: <5ms
    group.bench_function("get_function_signature", |b| {
        b.iter(|| {
            let signature = parser.get_signature(&ast, "new");
            black_box(signature);
        });
    });

    group.finish();
}

// ==============================================================================
// AST Editing Benchmarks
// ==============================================================================

fn bench_ast_editing(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_editing");
    group.significance_level(0.05).sample_size(50);

    let base_code = generate_complex_rust_module(500);

    // Add function - Target: <20ms
    group.bench_function("add_function", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let new_code = "pub fn new_function(x: i32) -> i32 { x * 2 }";
            let result = editor.add_function(&base_code, new_code, None).unwrap();
            black_box(result);
        });
    });

    // Rename identifier - Target: <50ms
    group.bench_function("rename_identifier", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let result = editor
                .rename_identifier(&base_code, "DataStruct0", "RenamedStruct")
                .unwrap();
            black_box(result);
        });
    });

    // Delete function - Target: <20ms
    group.bench_function("delete_function", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let result = editor.delete_function(&base_code, "new").unwrap();
            black_box(result);
        });
    });

    // Modify function body - Target: <30ms
    group.bench_function("modify_function_body", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let new_body = "{\n\tprintln!(\"Modified\");\n\tSelf::default()\n}";
            let result = editor
                .replace_function_body(&base_code, "new", new_body)
                .unwrap();
            black_box(result);
        });
    });

    // Add parameter to function - Target: <30ms
    group.bench_function("add_function_parameter", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let result = editor
                .add_parameter(&base_code, "new", "extra: String")
                .unwrap();
            black_box(result);
        });
    });

    // Extract method (complex refactoring) - Target: <100ms
    group.bench_function("extract_method", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let code_to_extract = "\t\tself.data.extend(data);";
            let result = editor
                .extract_method(&base_code, code_to_extract, "extend_data", &["data"])
                .unwrap();
            black_box(result);
        });
    });

    // Add import statement - Target: <15ms
    group.bench_function("add_import", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let result = editor
                .add_import(&base_code, "use std::path::PathBuf;")
                .unwrap();
            black_box(result);
        });
    });

    // Inline variable - Target: <40ms
    group.bench_function("inline_variable", |b| {
        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let result = editor
                .inline_variable(&base_code, "result")
                .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// ==============================================================================
// Code Generation Benchmarks
// ==============================================================================

fn bench_code_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("code_generation");
    group.significance_level(0.05).sample_size(100);

    // Generate simple function - Target: <5ms
    group.bench_function("generate_simple_function", |b| {
        b.iter(|| {
            let code = generate_rust_code(4);
            black_box(code);
        });
    });

    // Generate 100 LOC - Target: <10ms
    group.throughput(Throughput::Elements(100));
    group.bench_function("generate_100_loc", |b| {
        b.iter(|| {
            let code = generate_rust_code(100);
            black_box(code);
        });
    });

    // Generate 1K LOC - Target: <100ms
    group.throughput(Throughput::Elements(1000));
    group.bench_function("generate_1000_loc", |b| {
        b.iter(|| {
            let code = generate_rust_code(1000);
            black_box(code);
        });
    });

    // Generate complex module - Target: <50ms
    group.bench_function("generate_complex_module", |b| {
        b.iter(|| {
            let code = generate_complex_rust_module(500);
            black_box(code);
        });
    });

    // Generate struct with methods - Target: <10ms
    group.bench_function("generate_struct_with_methods", |b| {
        b.iter(|| {
            let code = format!(
                "#[derive(Debug, Clone)]\n\
                 pub struct MyStruct {{\n\
                 \tpub field1: String,\n\
                 \tpub field2: i32,\n\
                 }}\n\n\
                 impl MyStruct {{\n\
                 \tpub fn new(field1: String, field2: i32) -> Self {{\n\
                 \t\tSelf {{ field1, field2 }}\n\
                 \t}}\n\
                 \n\
                 \tpub fn get_field1(&self) -> &str {{\n\
                 \t\t&self.field1\n\
                 \t}}\n\
                 }}"
            );
            black_box(code);
        });
    });

    // Generate trait implementation - Target: <15ms
    group.bench_function("generate_trait_impl", |b| {
        b.iter(|| {
            let code = format!(
                "impl MyTrait for MyStruct {{\n\
                 \tfn method1(&self) -> String {{\n\
                 \t\tself.field1.clone()\n\
                 \t}}\n\
                 \n\
                 \tfn method2(&mut self, value: i32) {{\n\
                 \t\tself.field2 = value;\n\
                 \t}}\n\
                 }}"
            );
            black_box(code);
        });
    });

    group.finish();
}

// ==============================================================================
// Dependency Extraction Benchmarks
// ==============================================================================

fn bench_dependency_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_extraction");
    group.significance_level(0.05).sample_size(100);

    let complex_code = generate_complex_rust_module(1000);
    let parser = Parser::new(Language::Rust);
    let ast = parser.parse(&complex_code).unwrap();

    // Extract imports - Target: <10ms
    group.bench_function("extract_imports", |b| {
        b.iter(|| {
            let extractor = DependencyExtractor::new();
            let imports = extractor.extract_imports(&ast);
            black_box(imports);
        });
    });

    // Extract function calls - Target: <20ms
    group.bench_function("extract_function_calls", |b| {
        b.iter(|| {
            let extractor = DependencyExtractor::new();
            let calls = extractor.extract_function_calls(&ast);
            black_box(calls);
        });
    });

    // Extract type references - Target: <20ms
    group.bench_function("extract_type_references", |b| {
        b.iter(|| {
            let extractor = DependencyExtractor::new();
            let types = extractor.extract_type_references(&ast);
            black_box(types);
        });
    });

    // Build full dependency graph - Target: <50ms
    group.bench_function("build_dependency_graph", |b| {
        b.iter(|| {
            let extractor = DependencyExtractor::new();
            let graph = extractor.build_dependency_graph(&ast);
            black_box(graph);
        });
    });

    // Find all references to symbol - Target: <30ms
    group.bench_function("find_symbol_references", |b| {
        b.iter(|| {
            let extractor = DependencyExtractor::new();
            let refs = extractor.find_references(&ast, "HashMap");
            black_box(refs);
        });
    });

    group.finish();
}

// ==============================================================================
// Batch Operations Benchmarks
// ==============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    group.significance_level(0.05).sample_size(20);
    group.measurement_time(Duration::from_secs(15));

    // Parse multiple files - Target: <500ms for 100 files
    group.throughput(Throughput::Elements(100));
    group.bench_function("parse_100_files", |b| {
        let files: Vec<String> = (0..100).map(|_| generate_rust_code(100)).collect();

        b.iter(|| {
            let parser = Parser::new(Language::Rust);
            let asts: Vec<_> = files
                .iter()
                .map(|code| parser.parse(code).unwrap())
                .collect();
            black_box(asts);
        });
    });

    // Batch rename across multiple files - Target: <1s for 10 files
    group.bench_function("batch_rename_10_files", |b| {
        let files: Vec<String> = (0..10).map(|_| generate_complex_rust_module(500)).collect();

        b.iter(|| {
            let editor = AstEditor::new(Language::Rust);
            let results: Vec<_> = files
                .iter()
                .map(|code| editor.rename_identifier(code, "DataStruct0", "RenamedStruct").unwrap())
                .collect();
            black_box(results);
        });
    });

    // Extract dependencies from multiple files - Target: <200ms for 50 files
    group.throughput(Throughput::Elements(50));
    group.bench_function("extract_deps_50_files", |b| {
        let files: Vec<String> = (0..50).map(|_| generate_complex_rust_module(300)).collect();
        let parser = Parser::new(Language::Rust);
        let asts: Vec<_> = files.iter().map(|code| parser.parse(code).unwrap()).collect();

        b.iter(|| {
            let extractor = DependencyExtractor::new();
            let all_deps: Vec<_> = asts
                .iter()
                .map(|ast| extractor.extract_imports(ast))
                .collect();
            black_box(all_deps);
        });
    });

    group.finish();
}

// ==============================================================================
// Main Benchmark Configuration
// ==============================================================================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_parsing_rust,
        bench_parsing_typescript,
        bench_ast_queries,
        bench_ast_editing,
        bench_code_generation,
        bench_dependency_extraction,
        bench_batch_operations,
);

criterion_main!(benches);
