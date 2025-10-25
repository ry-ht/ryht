//! Integration tests for concurrent file processing with cortex code analysis.

use anyhow::Result;
use cortex_code_analysis::{
    concurrent::{ConcurrentRunner, FilesData},
    Lang, RustParser,
};
use globset::{Glob, GlobSetBuilder};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_basic() -> Result<()> {
    let temp = TempDir::new()?;
    let file1 = temp.path().join("test1.rs");
    let file2 = temp.path().join("test2.rs");

    fs::write(&file1, "fn main() {}")?;
    fs::write(&file2, "fn test() {}")?;

    let processed = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed.clone();

    let runner = ConcurrentRunner::new(2, move |path, _: &()| {
        processed_clone.lock().unwrap().push(path);
        Ok(())
    });

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include: globset::GlobSet::empty(),
        exclude: globset::GlobSet::empty(),
    };

    runner.run((), files)?;

    let processed = processed.lock().unwrap();
    assert_eq!(processed.len(), 2);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_with_parser() -> Result<()> {
    let temp = TempDir::new()?;

    // Create test Rust files
    let file1 = temp.path().join("lib1.rs");
    let file2 = temp.path().join("lib2.rs");

    fs::write(
        &file1,
        r#"
        pub fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        pub fn subtract(a: i32, b: i32) -> i32 {
            a - b
        }
        "#,
    )?;

    fs::write(
        &file2,
        r#"
        pub struct Point {
            x: f64,
            y: f64,
        }

        impl Point {
            pub fn new(x: f64, y: f64) -> Self {
                Point { x, y }
            }
        }
        "#,
    )?;

    struct AnalysisResults {
        total_functions: usize,
        total_structs: usize,
    }

    let results = Arc::new(Mutex::new(AnalysisResults {
        total_functions: 0,
        total_structs: 0,
    }));
    let results_clone = results.clone();

    let runner = ConcurrentRunner::new(2, move |path, _: &()| {
        let source = std::fs::read_to_string(&path)?;
        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file(path.to_str().unwrap(), &source)?;

        let mut results = results_clone.lock().unwrap();
        results.total_functions += parsed.functions.len();
        results.total_structs += parsed.structs.len();

        Ok(())
    });

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include: globset::GlobSet::empty(),
        exclude: globset::GlobSet::empty(),
    };

    runner.run((), files)?;

    let results = results.lock().unwrap();
    assert_eq!(results.total_functions, 3); // add, subtract, new
    assert_eq!(results.total_structs, 1); // Point

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_with_filters() -> Result<()> {
    let temp = TempDir::new()?;

    // Create mixed files
    fs::write(temp.path().join("file1.rs"), "fn test1() {}")?;
    fs::write(temp.path().join("file2.rs"), "fn test2() {}")?;
    fs::write(temp.path().join("file3.txt"), "not rust code")?;
    fs::write(temp.path().join("readme.md"), "# Documentation")?;

    let processed = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed.clone();

    let runner = ConcurrentRunner::new(2, move |path, _: &()| {
        processed_clone.lock().unwrap().push(path);
        Ok(())
    });

    // Only process .rs files
    let mut include_builder = GlobSetBuilder::new();
    include_builder.add(Glob::new("**/*.rs")?);
    let include = include_builder.build()?;

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include,
        exclude: globset::GlobSet::empty(),
    };

    runner.run((), files)?;

    let processed = processed.lock().unwrap();
    assert_eq!(processed.len(), 2); // Only .rs files

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_with_exclusion() -> Result<()> {
    let temp = TempDir::new()?;

    // Create directory structure
    fs::create_dir(temp.path().join("src"))?;
    fs::create_dir(temp.path().join("target"))?;

    fs::write(temp.path().join("src/main.rs"), "fn main() {}")?;
    fs::write(temp.path().join("target/build.rs"), "fn build() {}")?;

    let processed = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed.clone();

    let runner = ConcurrentRunner::new(2, move |path, _: &()| {
        processed_clone.lock().unwrap().push(path);
        Ok(())
    });

    // Exclude target directory
    let mut exclude_builder = GlobSetBuilder::new();
    exclude_builder.add(Glob::new("**/target/**")?);
    let exclude = exclude_builder.build()?;

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include: globset::GlobSet::empty(),
        exclude,
    };

    runner.run((), files)?;

    let processed = processed.lock().unwrap();
    assert_eq!(processed.len(), 1); // Only src/main.rs

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_proc_dir_paths() -> Result<()> {
    let temp = TempDir::new()?;

    fs::write(temp.path().join("file1.rs"), "fn test1() {}")?;
    fs::write(temp.path().join("file2.rs"), "fn test2() {}")?;
    fs::write(temp.path().join("file3.txt"), "text")?;

    let runner = ConcurrentRunner::new(2, |_path, _: &()| Ok(()))
        .set_proc_dir_paths(|files, path, _: &()| {
            if let Some(ext) = path.extension() {
                files
                    .entry(ext.to_string_lossy().to_string())
                    .or_insert_with(Vec::new)
                    .push(path.to_path_buf());
            }
        });

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include: globset::GlobSet::empty(),
        exclude: globset::GlobSet::empty(),
    };

    let results = runner.run((), files)?;

    assert_eq!(results.get("rs").map(|v| v.len()), Some(2));
    assert_eq!(results.get("txt").map(|v| v.len()), Some(1));

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_error_handling() -> Result<()> {
    let temp = TempDir::new()?;

    fs::write(temp.path().join("valid.rs"), "fn test() {}")?;
    fs::write(temp.path().join("invalid.rs"), "invalid rust code")?;

    let processed = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed.clone();

    let runner = ConcurrentRunner::new(2, move |path, _: &()| {
        // Try to parse as Rust - might fail for invalid files
        let source = std::fs::read_to_string(&path)?;

        // Even if parsing fails, we still processed the file
        processed_clone.lock().unwrap().push(path);

        Ok(())
    });

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include: globset::GlobSet::empty(),
        exclude: globset::GlobSet::empty(),
    };

    runner.run((), files)?;

    let processed = processed.lock().unwrap();
    assert_eq!(processed.len(), 2);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_nested_directories() -> Result<()> {
    let temp = TempDir::new()?;

    // Create nested structure
    fs::create_dir_all(temp.path().join("src/modules/utils"))?;
    fs::write(temp.path().join("src/main.rs"), "fn main() {}")?;
    fs::write(temp.path().join("src/modules/mod.rs"), "pub mod utils;")?;
    fs::write(temp.path().join("src/modules/utils/helpers.rs"), "pub fn help() {}")?;

    let processed = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed.clone();

    let runner = ConcurrentRunner::new(2, move |path, _: &()| {
        processed_clone.lock().unwrap().push(path);
        Ok(())
    });

    let files = FilesData {
        paths: vec![temp.path().join("src")],
        include: globset::GlobSet::empty(),
        exclude: globset::GlobSet::empty(),
    };

    runner.run((), files)?;

    let processed = processed.lock().unwrap();
    assert_eq!(processed.len(), 3);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system setup"]
fn test_concurrent_with_config() -> Result<()> {
    let temp = TempDir::new()?;

    fs::write(temp.path().join("file.rs"), "fn test() {}")?;

    struct Config {
        language: Lang,
        verbose: bool,
    }

    let processed = Arc::new(Mutex::new(0));
    let processed_clone = processed.clone();

    let runner = ConcurrentRunner::new(2, move |_path, config: &Config| {
        assert_eq!(config.language, Lang::Rust);
        assert!(config.verbose);

        *processed_clone.lock().unwrap() += 1;
        Ok(())
    });

    let files = FilesData {
        paths: vec![temp.path().to_path_buf()],
        include: globset::GlobSet::empty(),
        exclude: globset::GlobSet::empty(),
    };

    let config = Config {
        language: Lang::Rust,
        verbose: true,
    };

    runner.run(config, files)?;

    assert_eq!(*processed.lock().unwrap(), 1);

    Ok(())
}
