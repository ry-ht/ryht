# Technical Appendix: Code References and Implementation Details

## File Location Reference Map

### Experimental Codebase Structure
```
experiments/adv-rust-code-analysis/src/
├── metrics/
│   ├── abc.rs              - ABC (Assignments, Branches, Conditions)
│   ├── cognitive.rs        - Cognitive Complexity
│   ├── cyclomatic.rs       - Cyclomatic Complexity
│   ├── exit.rs             - Exit Points
│   ├── halstead.rs         - Halstead Metrics (17 public items - KEY TARGET)
│   ├── loc.rs              - Lines of Code (42 public items - most comprehensive)
│   ├── mi.rs               - Maintainability Index
│   ├── nargs.rs            - Number of Arguments
│   ├── nom.rs              - Number of Methods
│   ├── npa.rs              - Number of Public Attributes
│   ├── npm.rs              - Number of Public Methods
│   ├── wmc.rs              - Weighted Methods per Class
│   └── mod.rs              - Metrics module exports
├── languages/
│   ├── language_cpp.rs     - C++ implementation
│   ├── language_ccomment.rs - [DEPRECATED] Comment-only parser
│   ├── language_java.rs    - Java implementation
│   ├── language_javascript.rs - JavaScript implementation
│   ├── language_kotlin.rs  - Kotlin implementation
│   ├── language_mozjs.rs   - [DEPRECATED] Firefox JS dialect
│   ├── language_preproc.rs - [PARTIAL] Preprocessor support
│   ├── language_python.rs  - Python implementation
│   ├── language_rust.rs    - Rust implementation (150+ tokens)
│   ├── language_tsx.rs     - TSX implementation
│   ├── language_typescript.rs - TypeScript implementation (200+ tokens)
│   └── mod.rs              - Languages module exports
├── analysis/
│   ├── checker.rs          - [EXPERIMENTAL] Macro-based function detection
│   ├── getter.rs           - [EXPERIMENTAL] Advanced getter implementations
├── concurrent_files.rs     - Producer-consumer threading model
├── ast.rs                  - Full AST representation with spans
├── comment_rm.rs           - Comment removal utilities
├── function.rs             - Function span detection
├── ops.rs                  - Operations analysis
├── spaces.rs               - Space metrics (per-function)
├── preproc.rs              - Preprocessor analysis
├── traits.rs               - Core trait definitions
├── parser.rs               - Generic parser interface
├── tools.rs                - Utility functions
└── lib.rs                  - Library exports
```

### Cortex Codebase Structure
```
cortex/cortex-code-analysis/src/
├── metrics/
│   ├── abc.rs              - ABC metrics
│   ├── cognitive.rs        - Cognitive complexity
│   ├── cyclomatic.rs       - Cyclomatic complexity
│   ├── exit.rs             - Exit points
│   ├── halstead.rs         - Halstead metrics (NEEDS ENHANCEMENT)
│   ├── loc.rs              - Lines of code
│   ├── mi.rs               - Maintainability index
│   ├── nargs.rs            - Number of arguments
│   ├── nom.rs              - Number of methods
│   ├── npa.rs              - Number of public attributes
│   ├── npm.rs              - Number of public methods
│   ├── strategy.rs         - STRATEGY PATTERN (Parallel, Incremental, Default)
│   ├── wmc.rs              - Weighted methods per class
│   └── mod.rs              - Metrics module exports
├── analysis/
│   ├── alterator.rs        - AST transformation with builder pattern
│   ├── cache.rs            - LRU cache system (3 caches: AST, Metrics, Search)
│   ├── checker.rs          - NodeChecker trait with language dispatch
│   ├── count.rs            - AstCounter with 6 filter types
│   ├── find.rs             - AstFinder with 5 filter types
│   ├── getter.rs           - NodeGetter trait with language implementations
│   ├── tools.rs            - Analysis tools
│   ├── types.rs            - HalsteadType and SpaceKind enums
│   └── mod.rs              - Analysis module exports
├── languages/
│   ├── cpp.rs              - C++ language
│   ├── java.rs             - Java language
│   ├── javascript.rs       - JavaScript language
│   ├── kotlin.rs           - Kotlin language
│   ├── python.rs           - Python language
│   ├── rust.rs             - Rust language
│   ├── typescript.rs       - TypeScript language
│   ├── tsx.rs              - TSX language
│   └── mod.rs              - Languages module exports
├── concurrent/
│   ├── sync_runner.rs      - Sync concurrent processing
│   ├── async_runner.rs     - Async concurrent processing (feature-gated)
│   └── mod.rs              - Concurrent module exports
├── ast_builder.rs          - AST construction with config
├── ast_editor.rs           - AST editing and transformation
├── comment_removal.rs      - Comment extraction and removal
├── dependency_extractor.rs - Dependency graph generation
├── extractor.rs            - High-level element extraction
├── function.rs             - Function detection with FunctionSpan
├── lang.rs                 - Language enum and identification
├── node.rs                 - AST node wrapper
├── ops.rs                  - Code operations
├── output/                 - Serialization and export
├── parser.rs               - Generic parser trait
├── preprocessor.rs         - C/C++ preprocessor analysis
├── rust_parser.rs          - Rust-specific parser
├── spaces.rs               - Function space metrics
├── traits.rs               - Core trait definitions
├── tree_sitter_wrapper.rs  - Tree-sitter abstraction
├── types.rs                - Type definitions
├── typescript_parser.rs    - TypeScript-specific parser
├── utils.rs                - Utility functions
└── lib.rs                  - Library exports

tests/
├── test_comprehensive_metrics.rs
├── test_language_parsers.rs
├── test_metrics.rs
├── test_extraction.rs
├── concurrent_integration.rs
├── test_ast_editor_e2e.rs
├── test_dependency_extraction.rs
└── [13 more test files] (9,405 lines total)
```

## Key Classes and Structures to Migrate

### 1. Halstead Metrics Enhancement

**Current Location (Experimental):**
```rust
// experiments/adv-rust-code-analysis/src/metrics/halstead.rs, lines 1-150
pub struct HalsteadMaps<'a> {
    pub operators: HashMap<u16, u64>,      // Operator frequency
    pub operands: HashMap<&'a [u8], u64>,  // Operand frequency
}

impl<'a> HalsteadMaps<'a> {
    pub fn most_frequent_operators(&self, limit: usize) -> Vec<(u16, u64)>
    pub fn most_frequent_operands(&self, limit: usize) -> Vec<(&'a [u8], u64)>
    pub fn unique_operator_count(&self) -> usize
    pub fn unique_operand_count(&self) -> usize
}
```

**Target Location (Cortex):**
```
cortex/cortex-code-analysis/src/metrics/halstead.rs
```

**Migration Notes:**
- Add frequency maps to `HalsteadStats` struct
- Implement all four methods above
- Update serialization to include frequency data
- Add tests for distribution analysis

### 2. Advanced Checker Implementation

**Current Location (Experimental):**
```rust
// experiments/adv-rust-code-analysis/src/checker.rs
macro_rules! check_if_func { ... }
macro_rules! check_if_arrow_func { ... }
// Uses node.count_specific_ancestors() with predicates
```

**Target Location (Cortex):**
```
cortex/cortex-code-analysis/src/analysis/checker.rs
cortex/cortex-code-analysis/src/node.rs
```

**Migration Notes:**
- Add `count_specific_ancestors()` method to Node
- Convert macros to functions in checker.rs
- Add language-specific dispatch for function detection
- Implement for JavaScript and TypeScript specifically

### 3. Getter Implementation Enhancement

**Current Location (Experimental):**
```rust
// experiments/adv-rust-code-analysis/src/getter.rs
pub trait Getter {
    fn get_operator_id_as_str(id: u16) -> &'static str
    fn get_space_kind(_node: &Node) -> SpaceKind
    fn get_op_type(_node: &Node) -> HalsteadType
}
```

**Target Location (Cortex):**
```
cortex/cortex-code-analysis/src/analysis/getter.rs
```

**Migration Notes:**
- Implement per-language operator ID to string mapping
- Add special case handling (Rust `||`, etc.)
- Full implementation for all 7+ languages
- Add tests for each language's operator mapping

### 4. Comment Classification

**Current Location (Experimental):**
```rust
// experiments/adv-rust-code-analysis/src/checker.rs (implicit in patterns)
// Languages specific doc comment detection
```

**Target Location (Cortex):**
```
cortex/cortex-code-analysis/src/analysis/checker.rs
```

**Migration Notes:**
- Add `is_doc_comment()` method
- Add `is_coding_declaration_comment()` method
- Implement per-language patterns
- Add tests for doc comment detection

## Performance-Critical Code Paths

### 1. Halstead Operator/Operand Collection

**Experimental approach:**
```rust
// Stack-based traversal with operator/operand classification
for node in walk_tree(root) {
    match get_op_type(node) {
        HalsteadType::Operator => {
            *maps.operators.entry(node.kind_id()).or_insert(0) += 1;
        }
        HalsteadType::Operand => {
            let content = node.text();
            *maps.operands.entry(content).or_insert(0) += 1;
        }
        _ => {}
    }
}
```

**Cortex migration target:**
- Integrate into metrics calculation pipeline
- Use caching to avoid recomputation
- Implement parallel collection for large files
- Add memory pooling for large codebases

### 2. Function Detection with Ancestor Context

**Experimental approach:**
```rust
// Complex ancestor counting
node.count_specific_ancestors(
    |n| matches!(n.kind_id(), VariableDeclarator | AssignmentExpression),
    |n| matches!(n.kind_id(), StatementBlock | ReturnStatement)
) > 0
```

**Cortex migration target:**
- Implement as utility method on Node
- Cache results for subtrees
- Use stack-based ancestor traversal
- Avoid deep recursion

## Test Coverage Strategy

### Priority Test Additions

1. **Halstead Frequency Analysis** (cortex/tests/)
   ```rust
   #[test]
   fn test_halstead_most_frequent_operators() { ... }
   
   #[test]
   fn test_halstead_most_frequent_operands() { ... }
   
   #[test]
   fn test_halstead_distribution_analysis() { ... }
   ```

2. **Advanced Function Detection** (cortex/tests/)
   ```rust
   #[test]
   fn test_javascript_arrow_function_detection() { ... }
   
   #[test]
   fn test_typescript_method_detection() { ... }
   
   #[test]
   fn test_python_nested_function_detection() { ... }
   ```

3. **Operator String Mapping** (cortex/tests/)
   ```rust
   #[test]
   fn test_rust_operator_mapping() { ... }
   
   #[test]
   fn test_cpp_operator_mapping() { ... }
   
   #[test]
   fn test_javascript_operator_mapping() { ... }
   ```

4. **Comment Classification** (cortex/tests/)
   ```rust
   #[test]
   fn test_doc_comment_detection() { ... }
   
   #[test]
   fn test_coding_declaration_comments() { ... }
   ```

## Dependency Analysis

### Required for Halstead Enhancement
- HashMap (std) - for frequency maps
- serde - for serialization
- num-format (already used) - for formatting

### Required for Advanced Checking
- No new external dependencies
- Uses existing tree-sitter infrastructure

### Required for Comment Classification
- regex or aho-corasick (already available)
- Lazy OnceLock patterns (already in use)

## Version Compatibility Notes

### Tree-Sitter Versions (from Cargo.toml)
- Experimental: tree-sitter = "=0.25.3"
- Cortex: Uses latest compatible version
- JavaScript: tree-sitter-typescript = "=0.23.2"

### Serde Compatibility
- Both use serde with derive feature
- Serialization format is compatible
- Can share struct definitions

## Build Configuration

### Feature Flags to Consider
- `async`: Enable async runner (cortex)
- `cache`: Enable caching (cortex)
- `experimental`: Include experimental features
- `metrics-detailed`: Include frequency analysis (new)

### Cargo.toml Additions Needed
```toml
[features]
default = ["cache"]
cache = []
async = ["tokio", "futures"]
metrics-detailed = []
```

## Error Handling Patterns

### Current Patterns
- Experimental: println! for errors, sometimes panics
- Cortex: anyhow::Result<T>, proper error propagation

### Migration Strategy
- Replace all println! error output
- Use anyhow::bail! for error cases
- Add context() calls for clarity
- Implement Result<T> returns throughout

## Documentation References

### Inline Documentation
- Add doc comments to all new methods
- Link to test examples in doc comments
- Reference metrics in documentation

### Examples to Create
- Halstead frequency analysis example
- Advanced function detection example
- Comment classification example
- Per-language operator mapping example

---

## Implementation Checklist by File

### experiments/adv-rust-code-analysis/src/
- [ ] metrics/halstead.rs - Extract HalsteadMaps implementation
- [ ] languages/*.rs - Extract operator string mappings
- [ ] checker.rs - Extract function detection macros
- [ ] getter.rs - Extract all language-specific getters
- [ ] preproc.rs - Review for migration to cortex

### cortex/cortex-code-analysis/src/
- [ ] metrics/halstead.rs - Add frequency maps
- [ ] analysis/checker.rs - Add advanced function detection
- [ ] analysis/getter.rs - Add all operator mappings
- [ ] node.rs - Add count_specific_ancestors method
- [ ] analysis/types.rs - Ensure all types defined

### cortex/cortex-code-analysis/tests/
- [ ] Create test_halstead_analysis.rs
- [ ] Create test_advanced_function_detection.rs
- [ ] Create test_operator_mapping.rs
- [ ] Create test_comment_classification.rs

---

**Last Updated:** 2025-10-25
**Status:** Analysis Complete - Ready for Implementation
