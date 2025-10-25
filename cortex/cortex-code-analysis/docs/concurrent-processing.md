# Concurrent File Processing

The `concurrent` module provides a high-performance, multi-threaded system for processing multiple source code files in parallel.

## Overview

The concurrent processing system uses a producer-consumer architecture:
- **Producer**: A single thread that walks the file system and discovers files
- **Consumers**: Multiple worker threads that process files in parallel

## Features

- **Multi-threaded processing**: Configurable number of worker threads
- **File filtering**: Include/exclude patterns using glob syntax
- **Hidden file exclusion**: Automatically skips hidden files and directories
- **Flexible callbacks**: Custom processing logic per file
- **Error handling**: Robust error reporting and recovery
- **Metadata collection**: Track files by type, path, or custom criteria

## Basic Usage

### Simple File Processing

```rust
use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};
use globset::GlobSet;
use std::path::PathBuf;

// Create a runner with 4 worker threads
let runner = ConcurrentRunner::new(4, |path, _config: &()| {
    let content = std::fs::read_to_string(&path)?;
    println!("Processing {:?} - {} bytes", path, content.len());
    Ok(())
});

// Process all files in src/
let files = FilesData {
    paths: vec![PathBuf::from("src")],
    include: GlobSet::empty(),
    exclude: GlobSet::empty(),
};

runner.run((), files)?;
```

### With Code Analysis

```rust
use cortex_code_analysis::{
    concurrent::{ConcurrentRunner, FilesData},
    RustParser,
};
use std::sync::{Arc, Mutex};

// Collect analysis results
let results = Arc::new(Mutex::new(Vec::new()));
let results_clone = results.clone();

let runner = ConcurrentRunner::new(4, move |path, _: &()| {
    let source = std::fs::read_to_string(&path)?;
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file(path.to_str().unwrap(), &source)?;

    results_clone.lock().unwrap().push((
        path.clone(),
        parsed.functions.len(),
    ));

    Ok(())
});
```

### File Filtering

Include only specific file types:

```rust
use globset::{Glob, GlobSetBuilder};

let mut include_builder = GlobSetBuilder::new();
include_builder.add(Glob::new("**/*.rs")?);
include_builder.add(Glob::new("**/*.toml")?);
let include = include_builder.build()?;

let files = FilesData {
    paths: vec![PathBuf::from(".")],
    include,
    exclude: GlobSet::empty(),
};
```

Exclude directories:

```rust
let mut exclude_builder = GlobSetBuilder::new();
exclude_builder.add(Glob::new("**/target/**")?);
exclude_builder.add(Glob::new("**/.git/**")?);
exclude_builder.add(Glob::new("**/node_modules/**")?);
let exclude = exclude_builder.build()?;

let files = FilesData {
    paths: vec![PathBuf::from(".")],
    include: GlobSet::empty(),
    exclude,
};
```

### Custom Configuration

Pass configuration to workers:

```rust
struct Config {
    language: Lang,
    max_file_size: usize,
    verbose: bool,
}

let runner = ConcurrentRunner::new(4, |path, config: &Config| {
    let metadata = std::fs::metadata(&path)?;

    if metadata.len() as usize > config.max_file_size {
        if config.verbose {
            println!("Skipping large file: {:?}", path);
        }
        return Ok(());
    }

    // Process file...
    Ok(())
});

let config = Config {
    language: Lang::Rust,
    max_file_size: 1024 * 1024, // 1MB
    verbose: true,
};

runner.run(config, files)?;
```

### Collecting Metadata

Track files by extension:

```rust
let runner = ConcurrentRunner::new(4, |_path, _: &()| Ok(()))
    .set_proc_dir_paths(|files, path, _: &()| {
        if let Some(ext) = path.extension() {
            files
                .entry(ext.to_string_lossy().to_string())
                .or_insert_with(Vec::new)
                .push(path.to_path_buf());
        }
    });

let results = runner.run((), files)?;

for (ext, paths) in results {
    println!("Found {} .{} files", paths.len(), ext);
}
```

### Progress Tracking

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let total = Arc::new(AtomicUsize::new(0));
let processed = Arc::new(AtomicUsize::new(0));

let total_clone = total.clone();
let processed_clone = processed.clone();

let runner = ConcurrentRunner::new(4, move |path, _: &()| {
    // Process file...
    processed_clone.fetch_add(1, Ordering::Relaxed);
    Ok(())
})
.set_proc_path(move |_path, _: &()| {
    total_clone.fetch_add(1, Ordering::Relaxed);
});

runner.run((), files)?;

println!(
    "Processed {}/{} files",
    processed.load(Ordering::Relaxed),
    total.load(Ordering::Relaxed)
);
```

## Advanced Patterns

### Multi-Language Analysis

```rust
struct AnalysisConfig {
    rust_parser: Arc<Mutex<RustParser>>,
    ts_parser: Arc<Mutex<TypeScriptParser>>,
}

let runner = ConcurrentRunner::new(4, |path, config: &AnalysisConfig| {
    let source = std::fs::read_to_string(&path)?;

    let lang = Lang::from_path(&path)
        .ok_or_else(|| anyhow::anyhow!("Unknown language"))?;

    match lang {
        Lang::Rust => {
            let mut parser = config.rust_parser.lock().unwrap();
            let parsed = parser.parse_file(path.to_str().unwrap(), &source)?;
            // Process Rust file...
        }
        Lang::TypeScript => {
            let mut parser = config.ts_parser.lock().unwrap();
            let parsed = parser.parse_file(path.to_str().unwrap(), &source)?;
            // Process TypeScript file...
        }
        _ => {}
    }

    Ok(())
});
```

### Error Collection

```rust
let errors = Arc::new(Mutex::new(Vec::new()));
let errors_clone = errors.clone();

let runner = ConcurrentRunner::new(4, move |path, _: &()| {
    let result = std::fs::read_to_string(&path);

    match result {
        Ok(content) => {
            // Process content...
        }
        Err(e) => {
            errors_clone.lock().unwrap().push((path.clone(), e));
        }
    }

    Ok(()) // Always return Ok to continue processing
});

runner.run((), files)?;

let errors = errors.lock().unwrap();
if !errors.is_empty() {
    println!("Encountered {} errors:", errors.len());
    for (path, error) in errors.iter() {
        println!("  {:?}: {}", path, error);
    }
}
```

## Performance Tips

1. **Thread Count**: Use `num_cpus::get()` for optimal thread count
2. **Batch Processing**: Process related files together when possible
3. **Memory Management**: Be mindful of memory usage with large codebases
4. **Error Handling**: Don't panic in callbacks - return errors instead
5. **File Filters**: Use exclude patterns to skip unnecessary directories

## Thread Safety

All callbacks must be:
- `Send + Sync`: Safe to send between threads
- `'static`: No non-static references

Configuration must be:
- `Send + Sync`: Shared safely between workers
- Wrapped in `Arc` for reference counting

Shared state should use:
- `Arc<Mutex<T>>` for mutable shared data
- `Arc<AtomicXxx>` for simple counters/flags
- Message passing via channels for complex coordination

## Examples

See:
- `examples/concurrent_simple.rs` - Basic usage
- `examples/concurrent_analysis.rs` - Full code analysis
- `tests/concurrent_integration.rs` - Integration tests
