# Claude Code SDK (cc-sdk) - Comprehensive Architecture Analysis

**Crate Location**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/crates/cc-sdk`

**Version**: 0.3.0 (Rust Edition 2024)

**Status**: Modernization in progress - "Phase 3/4" architecture with both modern and legacy APIs

---

## 1. CURRENT MODULE STRUCTURE (src/)

### Core Directory Organization

```
src/
├── lib.rs                         # Main entry point and re-exports (198 lines)
├── core.rs                        # Core types and type-state pattern (579 lines)
├── error.rs                       # Modern error module (21 lines - thin re-export)
├── result.rs                      # Result type alias (35 lines)
├── errors.rs                      # Legacy error types (873 lines) [DUPLICATION]
│
├── client/                        # Phase 3: Modern type-safe client API
│   ├── mod.rs                     # Module re-exports
│   └── modern.rs                  # ClaudeClient with type-states (500+ lines)
│
├── session/                       # Phase 4: Session management
│   ├── mod.rs                     # API re-exports
│   ├── manager.rs                 # Project/session discovery (10K+ lines)
│   └── types.rs                   # Session data structures (3.4K lines)
│
├── settings/                      # Phase 4: Settings management
│   ├── mod.rs                     # API re-exports
│   ├── loader.rs                  # File I/O and merging (5.4K lines)
│   └── types.rs                   # ClaudeSettings structure (6.5K lines)
│
├── mcp/                           # Phase 4: MCP integration
│   └── mod.rs                     # Re-export mcp-sdk + helpers (3.1K lines)
│
├── binary/                        # Binary discovery and management
│   ├── mod.rs                     # Main discovery functions
│   ├── discovery.rs               # Discovery strategies
│   ├── cache.rs                   # Caching mechanisms
│   ├── env.rs                     # Environment handling
│   └── version.rs                 # Version parsing
│
├── transport/                     # Transport layer abstractions
│   ├── mod.rs                     # Transport trait (150 lines)
│   ├── subprocess.rs              # Subprocess transport implementation
│   └── mock.rs                    # Mock transport for testing
│
├── types.rs                       # Message types and options (1411 lines) [LARGE]
├── query.rs                       # Simple query interface (437 lines)
├── internal_query.rs              # Internal query handler (853 lines)
├── interactive.rs                 # Legacy interactive client (262 lines)
├── client_legacy.rs               # Deprecated client (791 lines) [LEGACY]
├── client_working.rs              # Working client variant (235 lines) [LEGACY]
├── optimized_client.rs            # Performance-optimized client (513 lines) [LEGACY]
│
├── token_tracker.rs               # Token usage tracking (346 lines)
├── model_recommendation.rs         # Model selection helpers (235 lines)
├── perf_utils.rs                  # Performance utilities (245 lines)
├── message_parser.rs              # Message parsing (409 lines)
│
├── sdk_mcp.rs                     # Legacy MCP server (377 lines) [DEPRECATED]
│
└── bin/
    └── test_interactive.rs        # Binary executable
```

### Size Analysis (Total: ~7.8K lines of Rust code)

| Category | Size | Count | Notes |
|----------|------|-------|-------|
| **Modern (Phase 3-4)** | ~2.0K | - | client/, session/, settings/, mcp/, core.rs |
| **Utility Modules** | ~2.3K | - | transport/, binary/, token_tracker, etc. |
| **Types & Serialization** | ~1.4K | 1 | types.rs (large monolithic file) |
| **Legacy/Deprecated** | ~1.6K | 5 | client_legacy, client_working, optimized_client, sdk_mcp, interactive |
| **Error Types** | ~0.9K | 2 | errors.rs (legacy) + error.rs (modern wrapper) |

---

## 2. PUBLIC API SURFACE

### Main Entry Points (lib.rs)

#### **Phase 3: Modern Client (Recommended)**
```rust
pub use client::{ClaudeClient, ClaudeClientBuilder, MessageStream};
```
- **Type-state pattern**: NoBinary → WithBinary → Configured → Connected → Disconnected
- Compile-time safe state transitions
- Built-in session management
- Ergonomic builder API

#### **Phase 4: Session Management (New)**
```rust
pub use session::{Project, Session, list_projects, list_sessions, load_session_history};
```
- Discover Claude projects
- List sessions per project
- Load JSONL-based session history
- Metadata extraction

#### **Phase 4: Settings Management (New)**
```rust
pub use settings::{ClaudeSettings, SettingsScope, load_settings, save_settings};
pub use prelude::settings::{Project, Session, list_projects, list_sessions};
```
- Multi-scope settings loading (User > Project > Local precedence)
- Hook configuration
- MCP server configuration
- Settings merging

#### **Phase 4: MCP Integration (New)**
```rust
pub use mcp::*;  // Re-export mcp-sdk
pub use sdk_mcp::{SdkMcpServer, SdkMcpServerBuilder, ToolDefinition, ToolHandler};
```
- Wraps mcp-sdk for in-process MCP servers
- Helper functions for config conversion
- Tool definition and execution

#### **Simple Query Interface**
```rust
pub use query::query;
```
- One-shot, stateless queries
- Simple unidirectional communication
- Good for scripts and batch operations

#### **Legacy Clients (Backward Compatibility)**
```rust
pub use client_legacy::ClaudeSDKClient;
pub use client_working::ClaudeSDKClientWorking;
pub use interactive::InteractiveClient;
pub use optimized_client::{OptimizedClient, ClientMode};
```

#### **Core Types**
```rust
pub use core::{BinaryPath, ModelId, SessionId, Version};
pub use error::{Error, BinaryError, ClientError, SessionError, SettingsError, TransportError};
pub use result::Result;
```

### Prelude Module (Convenience)
```rust
pub mod prelude {
    // Modern (Phase 1-4)
    pub use crate::{
        BinaryPath, Error, ModelId, Result, SessionId, Version,
        ClaudeClient, ClaudeClientBuilder, MessageStream,
        Project, Session, list_projects, list_sessions, load_session_history,
        ClaudeSettings, SettingsScope, load_settings, save_settings,
    };
    
    // Legacy (backward compat)
    pub use crate::{
        ClaudeCodeOptions, ClaudeSDKClientWorking, Message, PermissionMode,
        LegacyResult, SdkError, query, ClaudeSDKClient,
    };
}
```

---

## 3. DEPRECATED & LEGACY CODE

### Critical: `sdk_mcp.rs` - **MARKED FOR REPLACEMENT**

**Status**: Explicitly marked as "legacy" in module docs
**Lines**: 377
**Issues**:
- Duplicate functionality with mcp-sdk integration
- Uses legacy error types (`SdkError`, `Result` from errors.rs)
- Standalone MCP implementation instead of using mcp-sdk
- No longer fits modern architecture (Phase 4)

**Replacement**: Should use `mcp` module which re-exports `mcp-sdk` with helpers

**Current Use**: Re-exported in lib.rs as:
```rust
pub use sdk_mcp::{
    SdkMcpServer, SdkMcpServerBuilder, ToolDefinition, ToolHandler,
    ToolInputSchema, ToolResult, create_simple_tool,
    ToolResultContent as SdkToolResultContent,
};
```

---

### Legacy Client Implementations (Deprecated)

#### 1. **`client_legacy.rs`** - ClaudeSDKClient (791 lines)
- **Status**: Backward compatibility only
- **Issues**: 
  - Complex state management with Arc<Mutex<>>
  - Multiple receivers pattern (message_tx, message_buffer)
  - Verbose session tracking
- **Should be replaced by**: ClaudeClient (modern, type-safe)

#### 2. **`client_working.rs`** - ClaudeSDKClientWorking (235 lines)
- **Status**: Alternate working implementation
- **Issues**: Redundant with client_legacy and ClaudeClient
- **Action**: Remove (consolidate into ClaudeClient)

#### 3. **`optimized_client.rs`** - OptimizedClient (513 lines)
- **Status**: Performance optimization attempt
- **Features**: Connection pooling, batch mode, semaphores
- **Issues**: 
  - Incomplete (many TODO markers)
  - Complexity without clear performance gains documented
  - Modern ClaudeClient could incorporate these patterns
- **Action**: Either complete or migrate patterns to ClaudeClient

#### 4. **`interactive.rs`** - InteractiveClient (262 lines)
- **Status**: Legacy interactive interface
- **Export**: `pub type ClaudeSDKClientDefault = InteractiveClient;`
- **Issues**: Less ergonomic than ClaudeClient
- **Action**: Deprecate with migration guide to ClaudeClient

---

### Error Type Duplication

**Problem**: Two error systems running in parallel

1. **`errors.rs`** (873 lines) - Legacy error types
   ```rust
   pub enum Error { /* 7+ variants */ }
   pub enum SdkError { /* 12+ variants */ }
   pub type Result<T> = std::result::Result<T, SdkError>;
   ```
   - Uses `thiserror::Error`
   - More variants but less organized
   - Used by: legacy clients, sdk_mcp.rs, transport

2. **`error.rs`** (21 lines) - Modern error types (wrapper)
   ```rust
   pub use crate::errors::*;  // Currently just re-exports!
   ```
   - Should contain: BinaryError, TransportError, SessionError, SettingsError, ClientError
   - Currently incomplete (thin re-export only)

**Action**: Consolidate - use error.rs structure but populate it fully

---

### Incomplete/Commented Out Code

In `lib.rs`:
```rust
// mod client_v2;  // Has compilation errors
// mod client_final;  // Has compilation errors
// pub use client_v2::ClaudeSDKClientV2;  // Has compilation errors
// pub use client_final::ClaudeSDKClientFinal;  // Has compilation errors
```

**Action**: Remove these files if they can't be compiled

---

## 4. CORE TYPES & DESIGN PATTERNS

### A. Type-State Pattern (Modern)

**File**: `core.rs` (579 lines)
**Pattern**: Phantom-type based compile-time state tracking

```rust
pub mod state {
    pub struct NoBinary;
    pub struct WithBinary;
    pub struct Configured;
    pub struct Connected;
    pub struct Disconnected;
}

pub struct ClaudeClient<State = Connected> {
    inner: Arc<ClientInner>,
    _state: PhantomData<State>,
}
```

**Progression**:
1. `NoBinary` - Initial state
2. `WithBinary` - After binary discovery
3. `Configured` - After configuration
4. `Connected` - After connection established
5. `Disconnected` - After disconnection

**Benefits**:
- Compile-time prevention of invalid operations
- Can't call `.send()` without being Connected
- Type-safe state transitions
- Zero runtime overhead (PhantomData)

**Modern Core Types**:
- `SessionId`: Newtype wrapper for session strings
- `BinaryPath`: Newtype wrapper for binary path
- `ModelId`: Newtype wrapper for model identifiers  
- `Version`: Semantic version with parsing and comparison

All have:
- Proper Display/Debug implementations
- From/AsRef conversions
- Serialization support

---

### B. Builder Pattern

**File**: `client/modern.rs` (500+ lines)
**Structure**:

```rust
// State-based builder
impl ClaudeClientBuilder<NoBinary> {
    pub async fn discover_binary(self) -> Result<ClaudeClientBuilder<WithBinary>>
    pub fn binary(self, path: impl Into<BinaryPath>) -> ClaudeClientBuilder<WithBinary>
}

impl ClaudeClientBuilder<WithBinary> {
    pub fn model(mut self, model: ModelId) -> Self
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self
    pub fn working_directory(mut self, path: impl Into<PathBuf>) -> Self
    pub fn add_allowed_tool(mut self, tool: String) -> Self
    pub fn configure(self) -> ClaudeClientBuilder<Configured>
}

impl ClaudeClientBuilder<Configured> {
    pub async fn connect(self) -> Result<ClaudeClientBuilder<Connected>>
}

impl ClaudeClientBuilder<Connected> {
    pub fn build(self) -> Result<ClaudeClient>
}
```

**Strengths**:
- Ergonomic fluent interface
- Type-enforced progression
- Clear separation of concerns
- Good error handling at each stage

---

### C. Session Management

**File**: `session/` (types.rs + manager.rs: ~13K lines)

**Key Types**:
```rust
pub struct Project {
    pub id: String,
    pub path: PathBuf,
    pub sessions: Vec<SessionId>,
}

pub struct Session {
    pub id: SessionId,
    pub project_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub first_message: Option<String>,
    pub file_path: Option<PathBuf>,
}

pub struct SessionMetadata {
    pub session_id: SessionId,
    pub created_at: DateTime<Utc>,
    pub first_message: Option<String>,
    pub message_count: usize,
    pub last_updated: DateTime<Utc>,
}
```

**Discovery Process**:
1. Find `~/.claude/projects/` directory
2. Each project has sessions in `<project>/sessions/`
3. Sessions stored as `.jsonl` files
4. Metadata extracted from JSONL headers
5. Can load full session history via `load_session_history()`

**Implementation Quality**:
- Uses blocking tasks for file I/O
- Proper error handling
- Graceful handling of missing directories
- JSONL parsing with line-by-line buffering

---

### D. Settings Management

**File**: `settings/` (types.rs + loader.rs: ~12K lines)

**Key Types**:
```rust
#[derive(Debug, Clone, Copy)]
pub enum SettingsScope {
    User,      // ~/.claude/settings.json
    Project,   // <project>/.claude/settings.json
    Local,     // ./.claude/settings.json
}

pub struct ClaudeSettings {
    pub hooks: HashMap<String, Vec<HookConfig>>,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub default_model: Option<String>,
    pub permission_mode: Option<String>,
    pub prompts: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub additional: HashMap<String, serde_json::Value>,
}

pub struct HookConfig {
    pub hook_type: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
}
```

**Precedence**: Local > Project > User

**Implementation Quality**:
- Proper precedence handling (reverse order loading)
- Blocking I/O in background tasks
- Serde for JSON serialization
- Handles missing scopes gracefully
- Directory creation for writes

---

### E. Message Types

**File**: `types.rs` (1411 lines - LARGE)

**Main Message Enum**:
```rust
pub enum Message {
    User { message: UserMessage, ... },
    Assistant { message: AssistantMessage, ... },
    System { subtype: String, ... },
    Result { ... },
}

pub struct UserMessage {
    pub content: Vec<ContentBlock>,
}

pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}

pub enum ContentBlock {
    Text(TextContent),
    ToolUse(ToolUseContent),
    ToolResult(ToolResultContent),
    Thinking(ThinkingContent),
}
```

**Configuration Types**:
```rust
pub struct ClaudeCodeOptions {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub permission_mode: PermissionMode,
    pub control_protocol_format: ControlProtocolFormat,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    pub hooks: Option<HashMap<String, Vec<HookCallback>>>,
    pub cwd: Option<PathBuf>,
    // ... more fields
}
```

**Issues with types.rs**:
- **Size**: 1411 lines - should be split
- **Organization**: Message types, config, hooks, permissions all mixed
- **Duplication**: Some types duplicated from settings/mcp/session modules
- **Recommendation**: Break into: messages.rs, options.rs, permissions.rs, hooks.rs

---

## 5. SESSION MANAGEMENT IMPLEMENTATION

**Location**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/crates/cc-sdk/src/session/`

### Architecture

```
manager.rs (10K lines)
├── get_claude_dir() → ~/.claude
├── get_projects_dir() → ~/.claude/projects
├── list_projects() → Async, blocking task
├── list_sessions() → Per-project sessions
├── list_sessions_sync() → Internal sync version
├── load_session_history() → Parse JSONL
└── parse_session_metadata() → Extract metadata

types.rs (3.4K lines)
├── Project
├── Session
└── SessionMetadata
```

### Key Features

1. **Project Discovery**
   - Scans ~/.claude/projects/
   - Reads project metadata from metadata.json
   - Associates sessions with projects

2. **Session Listing**
   - Finds .jsonl files in project/sessions/
   - Extracts creation timestamp
   - Stores first user message

3. **History Loading**
   - Line-by-line JSONL parsing
   - Extracts message type, role, content
   - Maintains order and relationships

4. **Async Design**
   - File I/O in blocking background tasks
   - Non-blocking error handling
   - Proper cleanup

### Strengths
- ✅ Proper async/await pattern
- ✅ Error propagation
- ✅ Handles missing directories gracefully
- ✅ JSONL format support

### Weaknesses
- ❌ No caching of discovery results
- ❌ No filtering or sorting options
- ❌ Limited metadata extraction
- ❌ No write operations (read-only)

---

## 6. MCP INTEGRATION APPROACH

**Location**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/crates/cc-sdk/src/mcp/`

### Current Implementation (Modern)

**File**: `mcp/mod.rs` (3.1K lines)

```rust
// Re-export entire mcp-sdk crate
pub use mcp_sdk::*;

// Convenience re-exports
pub use mcp_sdk::{
    error::{McpError, ToolError, ResourceError, TransportError},
    protocol::{JsonRpcRequest, JsonRpcResponse, ServerCapabilities},
    server::{McpServer, ServerBuilder},
    tool::{Tool, ToolContext, ToolResult},
    transport::Transport,
    PROTOCOL_VERSION,
};
```

**Helper Functions**:

```rust
pub fn config_to_server_builder(config: &McpServerConfig) -> Result<ServerBuilder>
pub fn create_sdk_server_config(
    name: impl Into<String>,
    server: std::sync::Arc<dyn std::any::Any + Send + Sync>,
) -> McpServerConfig
```

### Legacy Implementation (Should Be Removed)

**File**: `sdk_mcp.rs` (377 lines)

Duplicates MCP functionality:
- Manual JSON-RPC implementation
- Tool registry and execution
- Message handling

**Status**: Marked as legacy in docs; should be removed in favor of mcp module

---

## 7. ERROR HANDLING PATTERNS

### Modern Approach (Recommended)

**File**: `error.rs` (currently thin wrapper)

```rust
#[derive(Debug, Error)]
pub enum Error {
    #[error("Binary error: {0}")]
    Binary(#[from] BinaryError),
    
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    
    #[error("Session error: {0}")]
    Session(#[from] SessionError),
    
    #[error("Settings error: {0}")]
    Settings(#[from] SettingsError),
    
    #[error("Client error: {0}")]
    Client(#[from] ClientError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
}
```

**Domain-Specific Error Types**:

```rust
pub enum BinaryError {
    NotFound { searched_paths: Vec<PathBuf> },
    VersionCheckFailed { ... },
    IncompatibleVersion { ... },
    SpawnFailed { ... },
    InvalidEnvVar { ... },
}

pub enum TransportError {
    Io(std::io::Error),
    Closed,
    InvalidMessage { reason: String, raw: String },
    ChannelError(String),
    Timeout { duration: Duration },
    // ... more
}

pub enum SessionError {
    HomeDirectoryNotFound,
    IoError(std::io::Error),
    ParseError { path: PathBuf, reason: String },
    // ...
}

pub enum SettingsError {
    ParseError { path: PathBuf, reason: String },
    WriteError { path: PathBuf, reason: String },
    InvalidScope { scope: String, reason: String },
    // ...
}

pub enum ClientError {
    NotSupported { feature: String },
    // ...
}
```

### Legacy Approach (Should Migrate Away)

**File**: `errors.rs` (873 lines)

```rust
pub enum Error {
    // 7+ variants mixing concerns
}

pub enum SdkError {
    // 12+ variants
}

pub type Result<T> = std::result::Result<T, SdkError>;
```

**Issues**:
- Monolithic enum
- Mixed concerns
- Less actionable error information
- Not pub visible from modern error module

---

## 8. TESTING COVERAGE & APPROACH

### Test Files (18 total)

**Location**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/crates/cc-sdk/tests/`

| File | Purpose | Status |
|------|---------|--------|
| `client_tests.rs` | Modern client API | ✅ Partial |
| `session_tests.rs` | Session management | ✅ Partial |
| `settings_tests.rs` | Settings loading | ✅ Partial |
| `binary_tests.rs` | Binary discovery | ✅ Partial |
| `token_optimization_test.rs` | Token tracking | ✅ Unit |
| `streaming_test.rs` | Message streaming | ✅ Unit |
| `mock_api_test.rs` | Mock transport | ✅ Unit |
| `e2e_control.rs` | Control protocol | ⚠️ E2E (needs binary) |
| `e2e_mcp.rs` | MCP integration | ⚠️ E2E (needs binary) |
| `e2e_hooks.rs` | Hook system | ⚠️ E2E (needs binary) |
| `integration_test.rs` | Integration | ⚠️ Incomplete |
| `integration_tests.rs` | Integration | ⚠️ Incomplete |

### Testing Patterns Observed

**Unit Tests** (use mock/test-only code):
```rust
#[test]
fn test_client_builder_creation() {
    let _builder = ClaudeClient::builder();
}

#[test]
fn test_type_states() {
    use cc_sdk::core::state::*;
    let _: PhantomData<NoBinary> = PhantomData;
}
```

**Async Tests** (use tokio-test):
```rust
#[tokio::test]
async fn test_session_discovery() {
    match list_projects().await {
        Ok(projects) => { /* verify */ },
        Err(e) => { /* handle gracefully */ },
    }
}
```

**E2E Tests** (marked `#[ignore]`):
```rust
#[tokio::test]
#[ignore = "Requires Claude binary to be installed"]
async fn test_real_query() { ... }
```

### Testing Gaps

- ❌ No property-based tests
- ❌ Limited parametric testing
- ❌ No benchmarks
- ❌ No fuzz testing
- ❌ Mock transport could be more comprehensive
- ❌ Settings merge logic lacks tests

---

## 9. EXAMPLES & PATTERNS

### Example Files (50+ examples)

**Location**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/crates/cc-sdk/examples/`

#### Modern Examples (Recommended)

1. **`modern_client.rs`** - Phase 3 API demo
   ```rust
   let client = ClaudeClient::builder()
       .discover_binary().await?
       .model(ModelId::from("claude-sonnet-4-5-20250929"))
       .permission_mode(PermissionMode::AcceptEdits)
       .configure()
       .connect().await?
       .build()?;
   
   let mut stream = client.send("What is 2+2?").await?;
   while let Some(msg) = stream.next().await {
       println!("{:?}", msg?);
   }
   client.disconnect().await?;
   ```

2. **`basic_client.rs`** - Legacy API example (still useful)
   ```rust
   let mut client = ClaudeSDKClient::new(options);
   client.connect(None).await?;
   client.send_request("What is 1+1?".to_string(), None).await?;
   let mut messages = client.receive_messages().await;
   while let Some(msg) = messages.next().await {
       match msg {
           Ok(Message::Assistant { message }) => { /* handle */ },
           // ...
       }
   }
   ```

#### Other Examples

- `batch_processor.rs` - Batch operations
- `control_protocol_demo.rs` - Control protocol
- `hooks_typed.rs` - Strongly-typed hooks
- `permission_modes.rs` - Permission examples
- `token_efficient.rs` - Token optimization
- `rest_api_server.rs` - REST wrapper

### Pattern Observations

**✅ Good Patterns**:
- Fluent builder for configuration
- Streaming for large responses
- Type-safe state transitions
- Proper error handling with ? operator
- Async/await consistently used

**⚠️ Inconsistencies**:
- Legacy examples mix with modern examples
- Some examples have conflicting names (e.g., `test_interactive`, `interactive`)
- Over 50 examples - hard to discover right one

---

## 10. DESIGN PATTERNS IDENTIFIED

### A. Newtypes for Type Safety
```rust
pub struct SessionId(String);
pub struct BinaryPath(PathBuf);
pub struct ModelId(String);
```
✅ Prevents mixing different string types
✅ Enables custom methods
✅ Zero runtime cost

### B. Type-State Pattern
```rust
pub struct ClaudeClient<State = Connected> {
    _state: PhantomData<State>,
}
```
✅ Compile-time state verification
✅ Zero runtime overhead
✅ Clear progression path

### C. Builder Pattern
```rust
ClaudeClient::builder()
    .binary(path)
    .model(id)
    .permission_mode(mode)
    .configure()
    .connect()
    .build()
```
✅ Ergonomic configuration
✅ Type-enforced progression
✅ Partial application support

### D. Async All The Way
- Uses tokio for async runtime
- Streams for message handling
- Blocking tasks for I/O
- Proper error propagation

### E. Trait Objects for Polymorphism
```rust
pub trait Transport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn send_message(&mut self, message: InputMessage) -> Result<()>;
    fn receive_messages(&mut self) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send>>;
    // ...
}
```
✅ SubprocessTransport, MockTransport implementations
✅ Testability

---

## 11. CRITICAL ISSUES & RECOMMENDATIONS

### Issue 1: Dual Error Systems (HIGH PRIORITY)

**Current State**:
- `errors.rs`: 873 lines of legacy error types
- `error.rs`: 21 lines (thin wrapper)
- Both systems used in different modules

**Recommendation**:
```
1. Populate error.rs with complete error types
2. Re-export from error.rs, not errors.rs
3. Migrate all modules to use error.rs
4. Deprecate errors.rs (keep for backward compat)
5. Plan removal in v1.0.0
```

---

### Issue 2: Large Monolithic types.rs (MEDIUM PRIORITY)

**Current**: 1411 lines in one file
**Contains**: Messages, config, permissions, hooks, MCP config

**Recommendation**: Split into:
```
types/
├── messages.rs      (Message, ContentBlock, etc.)
├── options.rs       (ClaudeCodeOptions, builder)
├── permissions.rs   (PermissionMode, PermissionBehavior, etc.)
├── hooks.rs         (HookCallback, HookContext, etc.)
├── content.rs       (TextContent, ToolUseContent, etc.)
└── mod.rs           (re-exports)
```

---

### Issue 3: Legacy Client Implementations (MEDIUM PRIORITY)

**Current**:
- client_legacy.rs (791 lines) - ClaudeSDKClient
- client_working.rs (235 lines) - ClaudeSDKClientWorking
- optimized_client.rs (513 lines) - OptimizedClient (incomplete)
- interactive.rs (262 lines) - InteractiveClient

**Recommendation**:
1. Keep ClaudeClient (modern) as primary
2. Evaluate if optimized_client patterns worth incorporating
3. Deprecate client_legacy, client_working, interactive
4. Create migration guide to ClaudeClient
5. Plan removal in v0.5.0

---

### Issue 4: sdk_mcp.rs Should Be Removed (MEDIUM PRIORITY)

**Current**:
- 377 lines of duplicate MCP functionality
- Uses legacy error types
- Explicitly marked as "legacy" in docs

**Recommendation**:
1. Verify no external users (unlikely, new code uses `mcp` module)
2. Remove sdk_mcp.rs
3. Move public types to mcp/types.rs if needed
4. Keep only in mcp module which re-exports mcp-sdk

---

### Issue 5: Code Organization - Examples (LOW PRIORITY)

**Current**: 50+ examples mixed in examples/ directory
**Issues**:
- Hard to find the right example
- Mix of modern and legacy patterns
- Some overlap (e.g., multiple "interactive" examples)

**Recommendation**:
1. Create examples/README.md with categorization
2. Tag examples: modern (recommended), legacy, advanced, special-use
3. Consolidate overlapping examples
4. ~15-20 focused, well-commented examples better than 50+ confusing ones

---

### Issue 6: Settings & Session Modules Incomplete (LOW PRIORITY)

**Session**:
- Read-only implementation
- No write operations
- Could cache discovery results

**Settings**:
- Limited validation
- No schema validation
- Could benefit from builder pattern

---

## 12. MISSING FEATURES & OPPORTUNITIES

### A. Documentation
- ❌ No architecture document
- ❌ Limited inline docs for complex modules
- ❌ No migration guide from old to modern API
- ✅ Good README with examples

### B. Testing
- ❌ No integration test suite (just E2E markers)
- ❌ No benchmarks
- ❌ No property-based testing
- ⚠️ Mock transport could be richer

### C. Features
- ❌ Settings write operations
- ❌ Session caching/indexing
- ❌ Message history search
- ❌ Performance profiling utilities
- ⚠️ Streaming input support (marked TODO)

### D. Error Handling
- ❌ No error recovery strategies
- ❌ No retry logic (built-in)
- ⚠️ Timeout handling exists but limited

---

## 13. ARCHITECTURE SUMMARY

### Clean (Modern, Phase 3-4)
- ✅ **client/** - Type-safe modern client with type-states
- ✅ **core/** - Well-designed type-safe newtypes and state markers
- ✅ **session/** - Clean session discovery and loading
- ✅ **settings/** - Proper multi-scope settings management
- ✅ **mcp/** - Proper integration with mcp-sdk

### Needs Improvement (Technical Debt)
- ⚠️ **types.rs** - Too large, should be split
- ⚠️ **errors.rs + error.rs** - Duplication and inconsistency
- ⚠️ **sdk_mcp.rs** - Redundant with mcp module
- ⚠️ **legacy clients** - Should be deprecated

### Solid (Utility/Support)
- ✅ **transport/** - Good abstraction layer
- ✅ **binary/** - Comprehensive discovery
- ✅ **query.rs** - Simple stateless interface
- ✅ **token_tracker.rs** - Good token management

---

## 14. RECOMMENDATIONS SUMMARY

### Immediate (v0.3.x)
1. Add deprecation notices to legacy clients
2. Populate error.rs with proper types (not re-export)
3. Remove sdk_mcp.rs if no external users
4. Create migration guide from old to modern API

### Short-term (v0.4.x)
1. Split types.rs into smaller modules
2. Complete error.rs migration (phase out errors.rs)
3. Add settings write operations
4. Improve test coverage (especially integration tests)

### Medium-term (v0.5.x)
1. Remove deprecated legacy clients
2. Complete optimized_client patterns or remove
3. Add streaming input support
4. Performance benchmarks

### Long-term (v1.0.0)
1. Remove legacy error system (errors.rs)
2. Clean up duplicate code
3. Consider if multiple client variants needed or just ClaudeClient

