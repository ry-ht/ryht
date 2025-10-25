# Concurrent File Processing Module

High-performance, multi-threaded file processor for code analysis.

## Quick Start

```rust
use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};
use globset::GlobSet;
use std::path::PathBuf;

// Process files with 4 workers
let runner = ConcurrentRunner::new(4, |path, _: &()| {
    let content = std::fs::read_to_string(&path)?;
    // Process file...
    Ok(())
});

let files = FilesData {
    paths: vec![PathBuf::from("src")],
    include: GlobSet::empty(),
    exclude: GlobSet::empty(),
};

runner.run((), files)?;
```

## Features

✅ Multi-threaded processing
✅ Glob-based filtering
✅ Hidden file exclusion
✅ Generic configuration
✅ Error resilience
✅ Metadata collection

## Documentation

- **Full Guide**: `docs/concurrent-processing.md`
- **Examples**: `examples/concurrent_*.rs`
- **Tests**: `tests/concurrent_integration.rs`

## API

```rust
// Create runner
ConcurrentRunner::new(num_jobs, process_fn)
    .set_proc_dir_paths(dir_fn)  // Optional
    .set_proc_path(path_fn)       // Optional
    .run(config, files)

// Configure files
FilesData {
    paths: Vec<PathBuf>,
    include: GlobSet,
    exclude: GlobSet,
}
```

## Examples

### Simple Processing
```rust
let runner = ConcurrentRunner::new(4, |path, _: &()| {
    println!("Processing: {:?}", path);
    Ok(())
});
```

### With Parser
```rust
use cortex_code_analysis::RustParser;

let runner = ConcurrentRunner::new(4, |path, _: &()| {
    let source = std::fs::read_to_string(&path)?;
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file(path.to_str().unwrap(), &source)?;
    // Use parsed data...
    Ok(())
});
```

### With Filtering
```rust
use globset::{Glob, GlobSetBuilder};

let mut include = GlobSetBuilder::new();
include.add(Glob::new("**/*.rs")?);

let mut exclude = GlobSetBuilder::new();
exclude.add(Glob::new("**/target/**")?);

let files = FilesData {
    paths: vec![PathBuf::from(".")],
    include: include.build()?,
    exclude: exclude.build()?,
};
```

## See Also

- [Full Documentation](../docs/concurrent-processing.md)
- [Integration Guide](../CONCURRENT_INTEGRATION.md)
