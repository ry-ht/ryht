//! Simple example of concurrent file processing.
//!
//! Run with: cargo run --example concurrent_simple

use anyhow::Result;
use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};
use globset::GlobSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    // Track which files we processed
    let processed_files = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed_files.clone();

    // Create a concurrent runner with 4 worker threads
    let runner = ConcurrentRunner::new(4, move |path, _config: &()| {
        // Simple processing: just count the lines
        let content = std::fs::read_to_string(&path)?;
        let line_count = content.lines().count();

        println!("  {:?} - {} lines", path.file_name().unwrap(), line_count);

        processed_clone.lock().unwrap().push(path);
        Ok(())
    });

    // Process all .rs files in the src directory
    let files = FilesData {
        paths: vec![PathBuf::from("src")],
        include: GlobSet::empty(),
        exclude: GlobSet::empty(),
    };

    println!("Processing files concurrently...\n");

    runner.run((), files)?;

    let count = processed_files.lock().unwrap().len();
    println!("\nProcessed {} files successfully!", count);

    Ok(())
}
