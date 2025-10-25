//! Example demonstrating concurrent file processing with cortex-code-analysis.
//!
//! This example shows how to use the concurrent processing system to analyze
//! multiple Rust files in parallel, extracting function information from each.

use anyhow::Result;
use cortex_code_analysis::{
    concurrent::{ConcurrentRunner, FilesData},
    Lang, RustParser,
};
use globset::{Glob, GlobSetBuilder};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Configuration for the concurrent file processor.
struct AnalysisConfig {
    /// Language being analyzed
    language: Lang,
    /// Whether to print verbose output
    verbose: bool,
}

/// Results from analyzing a file.
#[derive(Debug)]
struct FileAnalysis {
    path: PathBuf,
    function_count: usize,
    line_count: usize,
}

fn main() -> Result<()> {
    println!("Concurrent Code Analysis Example");
    println!("=================================\n");

    // Collect results from all files
    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();

    // Create concurrent runner with 4 worker threads
    let runner = ConcurrentRunner::new(4, move |path, config: &AnalysisConfig| {
        if config.verbose {
            println!("Processing: {:?}", path);
        }

        // Read file content
        let source = std::fs::read_to_string(&path)?;

        // Parse the file
        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file(
            path.to_str().unwrap_or("unknown"),
            &source,
        )?;

        // Count lines
        let line_count = source.lines().count();

        // Store results
        let analysis = FileAnalysis {
            path: path.clone(),
            function_count: parsed.functions.len(),
            line_count,
        };

        results_clone.lock().unwrap().push(analysis);

        Ok(())
    })
    .set_proc_dir_paths(|files, path, _config| {
        // Group files by extension
        if let Some(ext) = path.extension() {
            files
                .entry(ext.to_string_lossy().to_string())
                .or_insert_with(Vec::new)
                .push(path.to_path_buf());
        }
    })
    .set_proc_path(|path, config: &AnalysisConfig| {
        if config.verbose {
            println!("Found: {:?}", path);
        }
    });

    // Build glob patterns for file filtering
    let mut include_builder = GlobSetBuilder::new();
    include_builder.add(Glob::new("**/*.rs")?);
    let include = include_builder.build()?;

    let mut exclude_builder = GlobSetBuilder::new();
    exclude_builder.add(Glob::new("**/target/**")?);
    exclude_builder.add(Glob::new("**/.git/**")?);
    let exclude = exclude_builder.build()?;

    // Configure which files to process
    let files_data = FilesData {
        paths: vec![PathBuf::from("src")],
        include,
        exclude,
    };

    // Configure the analysis
    let config = AnalysisConfig {
        language: Lang::Rust,
        verbose: true,
    };

    println!("Starting concurrent analysis...\n");

    // Run the concurrent analysis
    let file_groups = runner.run(config, files_data)?;

    println!("\n=================================");
    println!("Analysis Complete!");
    println!("=================================\n");

    // Print file groups
    println!("Files by extension:");
    for (ext, files) in &file_groups {
        println!("  .{}: {} files", ext, files.len());
    }
    println!();

    // Print analysis results
    let results = results.lock().unwrap();
    println!("Analyzed {} files:\n", results.len());

    let mut total_functions = 0;
    let mut total_lines = 0;

    for analysis in results.iter() {
        println!("  {:?}", analysis.path.file_name().unwrap_or_default());
        println!("    Functions: {}", analysis.function_count);
        println!("    Lines: {}", analysis.line_count);

        total_functions += analysis.function_count;
        total_lines += analysis.line_count;
    }

    println!("\nTotals:");
    println!("  Files: {}", results.len());
    println!("  Functions: {}", total_functions);
    println!("  Lines: {}", total_lines);
    println!("  Avg functions/file: {:.1}", total_functions as f64 / results.len() as f64);
    println!("  Avg lines/file: {:.1}", total_lines as f64 / results.len() as f64);

    Ok(())
}
