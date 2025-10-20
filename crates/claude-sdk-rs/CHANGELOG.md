# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2025-06-21

### 🛡️ Security Enhancement Release

This release significantly improves the security validation system to reduce false positives while maintaining protection against malicious input.

### ✨ New Features

#### Configurable Security Validation Levels
- **`SecurityLevel` enum**: Four distinct security levels for different use cases
  - `Strict`: Blocks most special characters (highest security)
  - `Balanced`: Context-aware validation (default, recommended)
  - `Relaxed`: Only blocks obvious attack patterns 
  - `Disabled`: No input validation (use with extreme caution)
- **Builder pattern support**: Set security level via `Client::builder().security_level(level)`
- **Per-client configuration**: Different security levels for different client instances

#### Context-Aware Validation (Balanced Mode)
- **Smart pattern detection**: Distinguishes legitimate use from actual attacks
- **Markdown support**: Allows backticks for code formatting (e.g., `"How do I use \`backticks\` in markdown?"`)
- **File operations**: Permits common file operations (e.g., `"create project-design-doc.md"`)
- **Git commands**: Allows legitimate git operations (e.g., `"git commit -m 'Initial commit'"`)
- **Safe redirection**: Context-aware detection of dangerous vs. safe command patterns

### 🔧 Improved Security Patterns

#### Enhanced Detection Logic
- **Command injection protection**: Improved detection of `$(...)`, `${...}`, `&&`, `||` patterns
- **Dangerous command detection**: Smart detection of risky commands after `;` and `|`
- **Multi-pattern analysis**: Requires multiple suspicious indicators before blocking
- **Reduced false positives**: Significantly fewer legitimate queries incorrectly flagged

#### Backwards Compatibility
- **Default behavior preserved**: `Balanced` mode is now default, maintaining security while reducing false positives
- **Existing API unchanged**: All existing client code continues to work without modification
- **Gradual adoption**: Users can opt into stricter or more relaxed modes as needed

### 🧪 Testing Improvements

#### Comprehensive Test Coverage
- **Security level testing**: Tests for all four security levels with representative queries
- **Edge case validation**: Specific tests for previously problematic patterns
- **Regression prevention**: Tests ensure the original issue (`"create project-design-doc.md"`) is resolved
- **Attack pattern verification**: Confirms malicious patterns are still properly blocked

### 📚 Documentation Updates

#### Enhanced README
- **Security section expansion**: Detailed explanation of security levels and their use cases
- **Configuration examples**: Practical examples showing how to configure different security levels
- **Migration guidance**: Clear guidance on when to use each security level
- **Security best practices**: Recommendations for different deployment scenarios

### 🔧 Technical Details

#### API Additions
```rust
// New security level enum
pub enum SecurityLevel {
    Strict,    // Blocks most special characters
    Balanced,  // Context-aware validation (default)
    Relaxed,   // Only obvious attacks
    Disabled,  // No validation
}

// New configuration methods
Config::builder().security_level(SecurityLevel::Balanced)
Client::builder().security_level(SecurityLevel::Relaxed)

// New validation function
validate_query_with_security_level(query, SecurityLevel::Balanced)
```

#### Internal Improvements
- **Modular validation logic**: Separate functions for different security levels
- **Optimized pattern matching**: Early returns for obviously safe patterns
- **Memory efficiency**: Reduced string allocations in validation logic

### 🛠️ Migration Guide

#### For Existing Users
- **No action required**: Default behavior is now more permissive while maintaining security
- **Custom security needs**: Users requiring stricter validation can opt into `SecurityLevel::Strict`
- **Trusted environments**: Users in controlled environments can use `SecurityLevel::Relaxed`

#### Configuration Examples
```rust
// For production with untrusted input
let client = Client::builder()
    .security_level(SecurityLevel::Strict)
    .build();

// For development and general use (default)
let client = Client::builder()
    .security_level(SecurityLevel::Balanced)
    .build();

// For trusted internal tools
let client = Client::builder()
    .security_level(SecurityLevel::Relaxed)
    .build();
```

### 🔒 Security Impact

#### Resolved Issues
- **False positive reduction**: Legitimate queries like `"create project-design-doc.md"` now work by default
- **Improved usability**: Better balance between security and functionality
- **Maintained protection**: All actual attack patterns continue to be blocked effectively

#### Security Matrix
| Pattern Type | Strict | Balanced | Relaxed | Disabled |
|--------------|--------|----------|---------|----------|
| `create file.md` | ❌ | ✅ | ✅ | ✅ |
| Backticks in markdown | ❌ | ✅ | ✅ | ✅ |
| `$(rm -rf /)` | ❌ | ❌ | ❌ | ✅ |
| `<script>alert()` | ❌ | ❌ | ❌ | ✅ |
| `'; DROP TABLE` | ❌ | ❌ | ❌ | ✅ |

### 🚀 Performance
- **Validation optimization**: Faster validation with early returns for safe patterns
- **Reduced overhead**: More efficient string operations in validation logic
- **Memory improvements**: Less temporary string allocation during validation

## [1.0.0] - 2025-06-19

### 🎉 Initial Stable Release

This is the first stable release of the claude-sdk-rs interactive CLI and SDK library. The project provides a comprehensive, type-safe, async-first Rust SDK for interacting with Claude AI through the Claude Code CLI.

### ✨ New Features

#### Core SDK (`claude-sdk-rs`)
- **Type-safe API**: Complete Rust wrapper for Claude Code CLI with strong typing
- **Three response modes**: Simple text, full metadata, and streaming responses
- **Builder pattern**: Fluent configuration with `Client::builder()` and `Config::builder()`
- **Session management**: Persistent session tracking with `SessionManager`
- **Tool integration**: Type-safe tool permissions and MCP server support
- **Async-first design**: Built on tokio for high-performance async operations
- **Error handling**: Comprehensive error types with proper error propagation

#### Interactive CLI (`claude-sdk-rs-interactive`)
- **Session management**: Create, switch, list, and delete Claude sessions
- **Command execution**: Run Claude commands with session context and parallel execution
- **Cost tracking**: Detailed cost analysis with breakdowns by command, session, and model
- **History management**: Searchable command history with advanced filtering
- **Analytics dashboard**: Usage insights and performance metrics
- **Data export**: Export cost and history data in JSON, CSV, and HTML formats
- **Configuration management**: Persistent configuration with file-based storage
- **Parallel execution**: Multi-agent parallel command execution for improved performance

#### Workspace Architecture
- **5-crate workspace**: Modular design with clear separation of concerns
  - `claude-sdk-rs`: Main SDK facade and public API
  - `claude-sdk-rs-core`: Core types, configuration, and session management
  - `claude-sdk-rs-runtime`: Process execution and Claude CLI interaction
  - `claude-sdk-rs-mcp`: Model Context Protocol implementation
  - `claude-sdk-rs-interactive`: Full-featured interactive CLI application

### ⚠️ Breaking Changes
- **Removed `claude-sdk-rs-macros` crate**: The procedural macros crate was removed as it contained only stub implementations. This crate provided no functionality and was removed to ensure v1.0.0 ships only production-ready code. Macros may be reintroduced in a future minor release if needed.

### 🛠️ Technical Features

#### Configuration System
- **Flexible config**: Support for TOML configuration files
- **CLI overrides**: Command-line arguments override file configuration
- **Environment variables**: Support for environment-based configuration
- **Default management**: Sensible defaults with easy customization
- **Validation**: Configuration validation with helpful error messages

#### Data Management
- **Persistent storage**: JSON-based storage for sessions, costs, and history
- **Search capabilities**: Advanced search with regex support and filtering
- **Data integrity**: Atomic operations and error recovery
- **Export functionality**: Multiple export formats for data portability
- **Backup support**: Easy backup and restore of all data

#### Performance & Reliability
- **Concurrent operations**: Thread-safe operations with proper synchronization
- **Error recovery**: Robust error handling with graceful degradation
- **Resource management**: Efficient memory usage and cleanup
- **Timeout handling**: Configurable timeouts for all operations
- **Streaming support**: Memory-efficient streaming for large responses

#### Developer Experience
- **Comprehensive docs**: Extensive documentation with examples
- **Rich examples**: Multiple example programs demonstrating key features
- **Integration tests**: Comprehensive test suite including CLI integration tests
- **Type safety**: Leverage Rust's type system for compile-time correctness
- **IDE support**: Full IDE support with proper type information

### 📊 CLI Commands

#### Session Management
- `session create <name>` - Create a new Claude session
- `session list` - List all sessions with optional detailed view
- `session switch <name>` - Switch to a different session
- `session delete <name>` - Delete a session with confirmation

#### Command Execution
- `run <command> [args...]` - Execute Claude commands with session context
- `run --parallel --agents <N>` - Parallel execution with multiple agents
- `list` - Discover and list available Claude commands

#### Analytics & Tracking
- `cost` - View cost summaries and breakdowns
- `cost --breakdown` - Detailed cost analysis by command and model
- `cost --export <format>` - Export cost data (JSON/CSV)
- `history` - Search and view command history
- `history --search <pattern>` - Search history with regex support
- `history --export <format>` - Export history data

#### Configuration
- `config show` - Display current configuration
- `config set <key> <value>` - Update configuration values
- `config reset` - Reset to default configuration
- `completion <shell>` - Generate shell completion scripts

### 🔧 Examples Included

- **basic.rs**: Simple SDK usage with text responses
- **streaming.rs**: Streaming response handling
- **with_tools.rs**: Tool integration and permissions
- **raw_json.rs**: Full JSON response access
- **simple.rs**: Minimal usage example
- **debug.rs**: Debug mode and error handling
- **profiling_example.rs**: Performance profiling and benchmarking

### 📦 Dependencies

#### External Requirements
- **Claude Code CLI**: Must be installed and authenticated
- **Rust 1.70+**: Minimum supported Rust version
- **Tokio runtime**: Async runtime for all operations

#### Key Crate Dependencies
- `tokio 1.40`: Async runtime and process spawning
- `serde 1.0`: Serialization framework
- `clap 4.5`: Command-line argument parsing
- `chrono 0.4`: Date and time handling
- `uuid 1.10`: Session and entry identification
- `thiserror 1.0`: Error handling and propagation

### 🧪 Testing

- **Unit tests**: Comprehensive unit test coverage for all modules
- **Integration tests**: End-to-end testing including CLI interactions
- **Property-based tests**: Using `proptest` for robust testing
- **Snapshot tests**: Using `insta` for response validation
- **Performance tests**: Benchmarking and performance regression detection

### 📋 Platform Support

- **Linux**: Full support on x86_64 and ARM64
- **macOS**: Full support on Intel and Apple Silicon
- **Windows**: Full support on x86_64

### 🔒 Security

- **Input validation**: Comprehensive input validation and sanitization
- **Safe subprocess execution**: Secure process spawning and management
- **Error information**: Careful error messages that don't leak sensitive data
- **File permissions**: Proper file permission handling
- **Dependency audit**: Regular security audits of dependencies

#### Known Security Advisories
- **RUSTSEC-2023-0071**: Medium severity RSA timing side-channel vulnerability in optional SQLite feature. This affects the `sqlite` feature only and does not impact core SDK functionality. Users not using the SQLite storage feature are unaffected.

### 🚀 Performance

- **Efficient async operations**: Non-blocking I/O for all network operations
- **Memory optimization**: Efficient memory usage with streaming support
- **Concurrent execution**: Support for parallel command execution
- **Caching**: Intelligent caching of session and configuration data
- **Resource cleanup**: Proper resource management and cleanup

### 📚 Documentation

- **API documentation**: Complete rustdoc documentation for all public APIs
- **User guides**: Comprehensive guides for CLI usage
- **Architecture docs**: Detailed architecture and design documentation
- **Examples**: Rich set of examples for common use cases
- **Tutorial**: Step-by-step tutorial for getting started

### 🔄 Migration Guide

This is the initial release, so no migration is needed. Future versions will include migration guides for breaking changes.

### 🔗 Links

- **Repository**: https://github.com/bredmond1019/claude-sdk-rust
- **Documentation**: https://docs.rs/claude-sdk-rs
- **Crates.io**: https://crates.io/crates/claude-sdk-rs
- **Issues**: https://github.com/bredmond1019/claude-sdk-rust/issues

### 🙏 Acknowledgments

- Built with the excellent Rust ecosystem
- Inspired by the Claude Code CLI tool
- Thanks to all contributors and testers

---

**Note**: This project wraps the official Claude Code CLI and requires it to be installed and authenticated. This is an unofficial community project and is not affiliated with Anthropic.

### What's Next?

Future releases will focus on:
- Enhanced MCP (Model Context Protocol) support
- Additional export formats and integrations
- Performance optimizations
- Extended tool ecosystem
- Community-requested features

Stay tuned for updates and please report any issues or feature requests on GitHub!