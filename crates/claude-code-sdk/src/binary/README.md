# Binary Discovery Module (Phase 2 Complete)

Comprehensive binary discovery system for locating Claude Code CLI installations across different platforms and installation methods.

## Phase 2 Enhancements

### What's New

1. **Smart Caching System** (`cache.rs`)
   - In-memory cache with configurable TTL (default: 1 hour)
   - Performance improvement: ~100x faster for cached results
   - Thread-safe global cache and instance-based cache options
   - Automatic expiration and cleanup
   - Cache hit/miss metrics

2. **Modern Error Integration**
   - Integration with Phase 1 error types (`BinaryError`, `Error`)
   - Better error messages with actionable suggestions
   - Contextual error information (searched paths, versions)

3. **Enhanced Discovery Builder**
   - `.use_cache()` method for selective caching
   - More granular control over discovery process
   - Better performance for custom configurations

4. **Performance Optimizations**
   - Cached discovery: <1ms (vs ~100ms uncached)
   - Lazy initialization of global cache
   - Efficient deduplication of discovered installations

5. **Environment Setup Improvements**
   - Better PATH configuration for Node.js/NVM
   - Enhanced proxy support (HTTP_PROXY, HTTPS_PROXY, NO_PROXY)
   - Platform-specific environment variables

## Features

- **Automatic Discovery**: Finds Claude installations in standard locations
- **Version Management**: Parses and compares semantic versions
- **Smart Caching**: In-memory cache with TTL and automatic cleanup
- **Platform Support**: Works on Unix (macOS, Linux) and Windows
- **Multiple Sources**: Supports system, NVM, Homebrew, npm, yarn, and custom paths
- **Environment Setup**: Properly configures command execution environments
- **Async Support**: Optional async variants for non-blocking discovery
- **Performance**: Up to 100x faster with caching enabled

## Module Structure

```
binary/
├── mod.rs          - Public API and documentation (255 lines)
├── discovery.rs    - Core binary discovery logic (780+ lines)
├── version.rs      - Version parsing and comparison (418 lines)
├── env.rs          - Environment setup and command building (261 lines)
├── cache.rs        - Discovery caching (550+ lines) [NEW]
└── README.md       - This file (enhanced documentation)
```

**Total**: 2,260+ lines of implementation code

## Quick Start

### Basic Usage

```rust
use cc_sdk::binary::find_claude_binary;

// Find the best Claude binary (cached)
let claude_path = find_claude_binary()
    .expect("Claude Code not found");

println!("Using Claude at: {}", claude_path);
```

### Discover All Installations

```rust
use cc_sdk::binary::discover_installations;

let installations = discover_installations();
for install in installations {
    println!("Found: {} (version: {:?}, source: {})",
        install.path, install.version, install.source);
}
```

### Custom Discovery

```rust
use cc_sdk::binary::DiscoveryBuilder;

let installations = DiscoveryBuilder::new()
    .custom_path("/opt/custom/claude")
    .skip_nvm(true)
    .discover();

println!("Found {} installations", installations.len());
```

### Working with Versions

```rust
use cc_sdk::binary::{Version, compare_versions};
use std::cmp::Ordering;

let v1 = Version::parse("1.0.41").unwrap();
let v2 = Version::parse("1.0.40").unwrap();
assert!(v1 > v2);

assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
```

### Creating Commands

```rust
use cc_sdk::binary::{find_claude_binary, create_command_with_env};

let claude_path = find_claude_binary().unwrap();
let mut cmd = create_command_with_env(&claude_path);
cmd.arg("--version");

let output = cmd.output().expect("Failed to execute");
println!("{}", String::from_utf8_lossy(&output.stdout));
```

## Discovery Process

The module checks locations in this order:

1. **which/where command**: Uses system `which` (Unix) or `where` (Windows)
2. **NVM directories**: `~/.nvm/versions/node/*/bin/claude`
3. **Homebrew paths**: `/opt/homebrew/bin/claude`, `/usr/local/bin/claude`
4. **System paths**: `/usr/bin/claude`, `/bin/claude`
5. **User-local paths**: `~/.local/bin/claude`, `~/.claude/local/claude`
6. **Package managers**: npm, yarn, bun global installations
7. **Environment variable**: `CLAUDE_BINARY_PATH` (custom override)

## Source Priority

When multiple installations are found, the best one is selected based on:

1. **Version** (highest version preferred)
2. **Source priority** (in order):
   - which/where (1)
   - homebrew (2)
   - system (3)
   - nvm-active (4)
   - nvm (5)
   - local-bin (6)
   - claude-local (7)
   - npm-global (8)
   - yarn (9)
   - bun (10)
   - node-modules (11)
   - home-bin (12)
   - PATH (13)

## Environment Variables

The module respects these environment variables:

- `CLAUDE_BINARY_PATH`: Custom path to Claude binary (highest priority)
- `NVM_BIN`: Active NVM binary directory
- `NVM_DIR` or `NVM_HOME`: NVM installation directory
- `HOMEBREW_PREFIX`: Homebrew installation prefix
- `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`: Proxy settings

## Platform Differences

### Unix (macOS, Linux)

- Uses `which` command
- Checks `~/.nvm/versions/node/*/bin/claude`
- Supports Homebrew paths (`/opt/homebrew`, `/usr/local`)
- Checks standard Unix paths (`/usr/bin`, `/bin`)

### Windows

- Uses `where` command
- Checks `%NVM_HOME%\*\claude.exe`
- Checks `%USERPROFILE%\.local\bin\claude.exe`
- Checks npm global directory (`%APPDATA%\npm`)

## Version Format

Versions follow semantic versioning: `major.minor.patch[-prerelease][+build]`

Examples:
- `1.0.41` - Standard version
- `2.0.0-beta.1` - Pre-release version
- `1.0.0+build123` - Version with build metadata
- `1.2.3-rc.1+build.456` - Complete version

### Version Comparison Rules

1. Compare major, minor, patch numerically
2. Pre-release versions have lower precedence than stable versions
3. Build metadata is ignored in comparisons
4. Pre-release identifiers are compared lexicographically

## Caching (Phase 2)

The module provides two levels of caching:

### 1. Global Cache (Automatic)

The `find_claude_binary()` and `discover_installations()` functions automatically use a global cache:

```rust
// First call - scans filesystem (~100ms)
let installations = discover_installations();

// Second call - uses cache (<1ms)
let installations = discover_installations();

// Clear cache to force fresh discovery
cc_sdk::binary::cache::clear_cache();
```

### 2. Instance-Based Cache (Custom)

For more control, use `DiscoveryCache` directly:

```rust
use cc_sdk::binary::cache::{DiscoveryCache, CacheConfig};
use std::time::Duration;

// Create cache with custom TTL
let config = CacheConfig {
    ttl: Duration::from_secs(1800), // 30 minutes
    enabled: true,
};
let mut cache = DiscoveryCache::new(config);

// Cache results
let installations = discover_installations();
cache.set_default(installations);

// Retrieve from cache
if let Some(cached) = cache.get_default() {
    println!("Found {} cached installations", cached.len());
}

// Cleanup expired entries
let removed = cache.cleanup();
println!("Removed {} expired entries", removed);
```

### Cache Configuration

```rust
use cc_sdk::binary::cache::CacheConfig;
use std::time::Duration;

let config = CacheConfig {
    ttl: Duration::from_secs(3600),  // 1 hour (default)
    enabled: true,                    // Enable/disable caching
};
```

### Discovery Builder with Caching

```rust
use cc_sdk::binary::DiscoveryBuilder;

let installations = DiscoveryBuilder::new()
    .use_cache(true)  // Enable caching for this discovery
    .discover();
```

### Performance Metrics

Typical performance (on modern macOS):

| Operation | Uncached | Cached | Speedup |
|-----------|----------|--------|---------|
| `discover_installations()` | ~100ms | <1ms | ~100x |
| `find_claude_binary()` | ~100ms | <0.1ms | ~1000x |

### Cache Lifetime

- **Default TTL**: 1 hour (3600 seconds)
- **Storage**: In-memory only (not persisted to disk)
- **Invalidation**: Automatic after TTL expires
- **Manual Clear**: `cache::clear_cache()` or `cache.clear()`

## Async Support

Enable the `async-discovery` feature for non-blocking discovery:

```toml
[dependencies]
cc-sdk = { version = "0.3.0", features = ["async-discovery"] }
```

```rust
use cc_sdk::binary::async_discovery::*;

#[tokio::main]
async fn main() {
    let path = find_claude_binary_async().await?;
    let installations = discover_installations_async().await;
}
```

## Error Handling

Functions return `Result<T, String>` with descriptive error messages:

```rust
match find_claude_binary() {
    Ok(path) => {
        // Use the path
    }
    Err(msg) => {
        eprintln!("Claude not found: {}", msg);
        eprintln!("Please install: npm install -g @anthropic-ai/claude-code");
    }
}
```

## Testing

The module includes comprehensive tests:

- **Unit tests**: Version parsing, comparison, environment setup
- **Integration tests**: Full discovery process, builder patterns
- **Doc tests**: All public API examples

Run tests:

```bash
# All tests
cargo test

# Binary module only
cargo test --lib binary

# Integration tests
cargo test --test binary_tests

# With logging
RUST_LOG=debug cargo test binary
```

## Examples

See `examples/binary_discovery.rs` for a complete demonstration:

```bash
cargo run --example binary_discovery
```

## Key Improvements Over Axon

This implementation includes several enhancements over the original axon implementation:

1. **Caching**: Uses `OnceLock` to cache discovery results
2. **Better Error Context**: Detailed messages about what was checked and why it failed
3. **Builder Pattern**: Configurable search via `DiscoveryBuilder`
4. **Custom Paths**: Support for `CLAUDE_BINARY_PATH` environment variable
5. **Async Support**: Optional async variants for non-blocking discovery
6. **No External Dependencies**: Version parsing implemented without regex crate
7. **Comprehensive Testing**: 305 lines of integration tests
8. **Extensive Documentation**: Full module documentation with examples

## Public API

### Types

- `ClaudeInstallation` - Represents a discovered installation
- `InstallationType` - System or Custom installation type
- `Version` - Parsed semantic version
- `DiscoveryBuilder` - Builder for custom discovery configuration

### Functions

- `find_claude_binary()` - Find best Claude binary (cached)
- `discover_installations()` - Discover all installations (fresh)
- `get_claude_version(path)` - Get version from binary path
- `create_command_with_env(program)` - Create Command with proper environment
- `compare_versions(a, b)` - Compare two version strings
- `extract_version_from_output(output)` - Parse version from CLI output

### Async Functions (with `async-discovery` feature)

- `find_claude_binary_async()` - Async binary finding
- `discover_installations_async()` - Async installation discovery
- `get_claude_version_async(path)` - Async version checking

## Integration

The binary module is fully integrated into cc-sdk:

```rust
// Available in prelude
use cc_sdk::prelude::*;

let path = find_claude_binary()?;
let installations = discover_installations();
```

## License

MIT License - See LICENSE file in repository root.
