# Strategy Library Lazy Initialization Fix

## Problem

When Axon MCP server starts in stdio mode with lazy Cortex initialization, it hangs during startup because:

1. `OrchestrateTool::new()` is called during MCP server build (axon/src/mcp_server/server.rs:67)
2. `OrchestrateTool::new()` calls `StrategyLibrary::new()` (axon/src/mcp_server/tools/orchestrate.rs:86)
3. `StrategyLibrary::new()` calls `load_learned_strategies()` (axon/src/orchestration/strategy_library.rs:251)
4. `load_learned_strategies()` tries to query Cortex via HTTP (axon/src/cortex_bridge/memory.rs:131-134)
5. If Cortex is not available yet, this waits for HTTP timeout (30 seconds), causing the MCP server initialization to hang

## Solution

Added a `lazy` parameter to `StrategyLibrary::new()` to skip loading learned strategies during initialization when in MCP stdio mode. The learned strategies can be loaded later on first use.

### Changes Made

#### 1. Updated `StrategyLibrary::new()` signature

**File:** `axon/src/orchestration/strategy_library.rs`

Added a third parameter `lazy: bool` to the constructor:

```rust
pub async fn new(
    cortex: Arc<CortexBridge>,
    config: StrategyLibraryConfig,
    lazy: bool,
) -> Result<Self>
```

When `lazy` is `true`:
- Only built-in default strategies are loaded during initialization
- Loading of learned strategies from Cortex is skipped
- Appropriate logging indicates lazy mode is active

#### 2. Made `load_learned_strategies()` public

**File:** `axon/src/orchestration/strategy_library.rs`

Changed visibility from private to public to allow manual loading after initialization:

```rust
pub async fn load_learned_strategies(&self) -> Result<()>
```

This method is now safe to call multiple times and includes proper error handling.

#### 3. Added `ensure_learned_strategies_loaded()` helper method

**File:** `axon/src/orchestration/strategy_library.rs`

Added a new public method for on-demand loading:

```rust
pub async fn ensure_learned_strategies_loaded(&self) -> Result<bool>
```

This method:
- Checks if learned strategies are already loaded (by looking for "learned_strategy_" prefix)
- Only loads if not already loaded (idempotent behavior)
- Returns `Ok(true)` if loaded, `Ok(false)` if already present
- Safe to call multiple times without side effects

#### 4. Updated `OrchestrateTool` to use lazy initialization

**File:** `axon/src/mcp_server/tools/orchestrate.rs`

Changed the `StrategyLibrary` initialization to use lazy mode:

```rust
// Initialize strategy library in lazy mode to avoid hanging on Cortex queries
// during MCP server initialization. Learned strategies will be loaded on first use.
let strategy_config = StrategyLibraryConfig::default();
let strategy_library = Arc::new(StrategyLibrary::new(cortex.clone(), strategy_config, true).await?);
```

#### 5. Added on-demand loading in `orchestrate()` method

**File:** `axon/src/mcp_server/tools/orchestrate.rs`

Added learned strategy loading when orchestration is first invoked:

```rust
// Load learned strategies now that Cortex is available (only loads once)
if let Err(e) = self.strategy_library.ensure_learned_strategies_loaded().await {
    // Log error but continue - we can still use default strategies
    tracing::warn!("Failed to load learned strategies: {}", e);
}
```

This ensures:
- Cortex is initialized before attempting to load strategies
- Learned strategies are available when needed
- Failures don't prevent orchestration (falls back to default strategies)
- Only loaded once due to idempotent check

#### 6. Updated example to maintain backward compatibility

**File:** `axon/examples/orchestrator_worker_demo.rs`

Updated to use `lazy: false` for immediate loading in non-MCP contexts:

```rust
// Use lazy: false to load learned strategies from Cortex immediately
let strategy_library = Arc::new(StrategyLibrary::new(cortex.clone(), strategy_config, false).await?);
```

## Benefits

1. **No more hanging during MCP server initialization** - The server can start immediately without waiting for Cortex
2. **Backward compatibility** - Existing code can continue using `lazy: false` for immediate loading
3. **Graceful degradation** - If learned strategy loading fails, the system continues with default strategies
4. **Efficient resource usage** - Strategies are only loaded when needed
5. **Idempotent loading** - Safe to call loading methods multiple times without side effects

## Testing

The implementation includes:

1. **Compilation verification** - All code compiles without errors
2. **Unit tests** - Basic tests verify strategy library functionality
3. **Integration tests** - Manual testing should verify:
   - MCP server starts without hanging in stdio mode
   - Orchestration works correctly with lazy-loaded strategies
   - Learned strategies are loaded on first orchestration call
   - System falls back to default strategies if Cortex is unavailable

## Files Modified

1. `/Users/taaliman/projects/luxquant/ry-ht/ryht/axon/src/orchestration/strategy_library.rs`
   - Added `lazy` parameter to `new()`
   - Made `load_learned_strategies()` public
   - Added `ensure_learned_strategies_loaded()` helper method
   - Added documentation and logging

2. `/Users/taaliman/projects/luxquant/ry-ht/ryht/axon/src/mcp_server/tools/orchestrate.rs`
   - Updated to use lazy initialization (`lazy: true`)
   - Added on-demand strategy loading in `orchestrate()` method

3. `/Users/taaliman/projects/luxquant/ry-ht/ryht/axon/examples/orchestrator_worker_demo.rs`
   - Updated to use non-lazy initialization (`lazy: false`)

## Migration Guide

For any other code calling `StrategyLibrary::new()`:

**Before:**
```rust
let library = StrategyLibrary::new(cortex, config).await?;
```

**After (for non-lazy contexts):**
```rust
let library = StrategyLibrary::new(cortex, config, false).await?;
```

**After (for lazy contexts like MCP stdio):**
```rust
let library = StrategyLibrary::new(cortex, config, true).await?;
// ... later, when Cortex is available ...
library.ensure_learned_strategies_loaded().await?;
```

## Next Steps

1. **Test manually** - Start the MCP server in stdio mode and verify no hanging occurs
2. **Test orchestration** - Verify that orchestration tasks work correctly with lazy-loaded strategies
3. **Monitor logs** - Check that appropriate log messages indicate lazy initialization and on-demand loading
4. **Consider adding metrics** - Track how often learned strategies are loaded vs. used

## Related Issues

This fix resolves the hanging issue described in the problem statement where MCP server initialization would timeout waiting for Cortex to respond during strategy library initialization.
