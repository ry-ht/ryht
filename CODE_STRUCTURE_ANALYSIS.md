# Code Structure Analysis Report
## Comparison: experiments/adv-rust-code-analysis vs cortex/cortex-code-analysis

**Generated:** 2025-10-25  
**Scope:** Architecture, modules, language support, and migration needs

---

## Executive Summary

The `cortex/cortex-code-analysis` directory contains a partially migrated and refactored version of the advanced analysis capabilities from `experiments/adv-rust-code-analysis`. However, significant features and sophisticated implementations from the experiments directory have not yet been fully integrated.

**Key Findings:**
- experiments has **52 source files** vs cortex has **47 source files**
- experiments has more complete metric implementations and language support infrastructure
- cortex has modern refactored architecture with better module organization
- critical missing: complete Kotlin/TSX language implementations in cortex
- experimental code has sophisticated concurrent processing, output formatting, and AST analysis capabilities

---

## 1. Module Structure Comparison

### experiments/adv-rust-code-analysis Modules:

```
src/
├── lib.rs (110 lines - core public API)
├── alterator.rs
├── ast.rs
├── c_langs_macros/
│   ├── c_macros.rs
│   ├── c_specials.rs
│   └── mod.rs
├── c_macro.rs
├── checker.rs (Complex language-specific node checking)
├── comment_rm.rs
├── concurrent_files.rs (Advanced concurrent processing)
├── count.rs
├── find.rs
├── function.rs
├── getter.rs (Language-specific node extraction)
├── langs.rs
├── languages/ (11 implementations including TSX, Kotlin)
│   ├── language_ccomment.rs
│   ├── language_cpp.rs
│   ├── language_java.rs
│   ├── language_javascript.rs
│   ├── language_kotlin.rs
│   ├── language_mozjs.rs (Mozilla-specific JS variant)
│   ├── language_preproc.rs
│   ├── language_python.rs
│   ├── language_rust.rs
│   ├── language_tsx.rs
│   ├── language_typescript.rs
│   └── mod.rs
├── macros.rs
├── metrics/
│   ├── abc.rs
│   ├── cognitive.rs
│   ├── cyclomatic.rs
│   ├── exit.rs
│   ├── halstead.rs
│   ├── loc.rs
│   ├── mi.rs
│   ├── mod.rs (no docs)
│   ├── nargs.rs
│   ├── nom.rs
│   ├── npa.rs
│   ├── npm.rs
│   └── wmc.rs
├── node.rs
├── ops.rs
├── output/ (3 modules - sophisticated output formatting)
│   ├── dump.rs
│   ├── dump_metrics.rs
│   ├── dump_ops.rs
│   └── mod.rs
├── parser.rs
├── preproc.rs
├── spaces.rs
├── tools.rs
├── traits.rs (Language-agnostic analysis traits)
└── comment_rm.rs
```

### cortex/cortex-code-analysis Modules:

```
src/
├── lib.rs (293 lines - comprehensive documented API)
├── lang.rs (Language enum with extensive support)
├── node.rs (Enhanced tree-sitter wrapper)
├── parser.rs
├── traits.rs (Simplified, improved trait design)
├── languages/ (8 implementations - modern structure)
│   ├── rust.rs (simplified token enum)
│   ├── typescript.rs
│   ├── javascript.rs
│   ├── python.rs
│   ├── cpp.rs (modern token-based approach)
│   ├── java.rs
│   ├── kotlin.rs (INCOMPLETE)
│   ├── tsx.rs (INCOMPLETE)
│   └── mod.rs
├── ast_builder.rs (New - modern AST building)
├── ast_editor.rs (New - AST manipulation)
├── comment_removal.rs
├── concurrent.rs (Simplified concurrency)
├── extractor.rs (Dependency extraction)
├── function.rs
├── rust_parser.rs (Rust-specific parser)
├── tree_sitter_wrapper.rs (Enhanced wrapper)
├── typescript_parser.rs (TypeScript-specific parser)
├── types.rs
├── dependency_extractor.rs
├── metrics/
│   ├── abc.rs
│   ├── cognitive.rs
│   ├── cyclomatic.rs
│   ├── exit.rs
│   ├── halstead.rs
│   ├── loc.rs
│   ├── mi.rs
│   ├── mod.rs (documented, 217 lines)
│   ├── nargs.rs
│   ├── nom.rs
│   ├── npa.rs
│   ├── npm.rs
│   └── wmc.rs
├── analysis/ (New - refactored analysis traits)
│   ├── checker.rs (NodeChecker trait + implementations)
│   ├── getter.rs (NodeGetter trait + implementations)
│   ├── types.rs
│   ├── tests.rs
│   └── mod.rs
├── ops.rs
├── preprocessor.rs
├── spaces.rs
├── utils.rs (New - utility functions)
└── concurrent.rs
```

---

## 2. Missing Modules in cortex/cortex-code-analysis

### High-Priority Missing:

1. **output/** module (3 files)
   - `dump.rs` - AST/code dumping utilities
   - `dump_metrics.rs` - Metrics output formatting
   - `dump_ops.rs` - Operations output formatting
   - **Impact:** Loss of formatted output capabilities

2. **Advanced C-language support**
   - `c_langs_macros/` module (preprocessor/macro handling)
   - `c_macro.rs` - C-specific macro handling
   - `language_mozjs.rs` - Mozilla-specific JavaScript dialect
   - `language_preproc.rs` - Preprocessor directives
   - **Impact:** Incomplete C/C++ analysis for preprocessor directives

3. **Tools/utilities**
   - `tools.rs` - General utilities
   - `find.rs` - AST searching utilities
   - `count.rs` - Counting utilities
   - **Impact:** Loss of utility functions for analysis

### Medium-Priority Missing:

4. **comment_rm.rs** (in experiments)
   - More sophisticated comment removal
   - **Impact:** Functionality exists in cortex but may be simpler

5. **parser.rs** differences
   - Filter logic and sophisticated parsing configuration
   - **Impact:** Limited customization in cortex

---

## 3. Language Support Status

### Fully Implemented in Both:
- Rust
- Python
- JavaScript
- TypeScript
- C++
- Java

### Incomplete/Missing in cortex:

| Language | experiments Status | cortex Status | Gap |
|----------|-------------------|---------------|-----|
| **Kotlin** | Full (language_kotlin.rs - complete) | **INCOMPLETE** | No Kotlin language implementation file content |
| **TSX** | Full (language_tsx.rs - ~300+ lines) | **INCOMPLETE** | tsx.rs exists but is incomplete |
| **Mozilla JS** | Full (language_mozjs.rs) | NOT PRESENT | No Mozilla-specific JS support |
| **Preprocessor** | Full (language_preproc.rs) | NOT PRESENT | No preprocessor directive support |
| **C-Comments** | Full (language_ccomment.rs) | PARTIAL | No dedicated C comment handling |

### Language-Specific Metrics:
- experiments: Each language has detailed trait implementations
- cortex: Better organized through analysis/checker.rs and analysis/getter.rs, but less complete

---

## 4. Detailed Feature Comparison

### A. Metrics Implementation

Both implement the same core metrics:
- **Complexity:** CC (Cyclomatic), Cognitive
- **Size:** LOC (SLOC, PLOC, LLOC, CLOC, Blank)
- **Halstead:** Complete vocabulary/volume metrics
- **Design:** ABC, WMC, NOM, NPM, NPA
- **Maintainability:** MI (Maintainability Index)
- **Other:** Exit points, NArgs

**Difference:** cortex has better documentation and a unified `CodeMetrics` struct (metrics/mod.rs lines 71-183), while experiments uses individual metric types scattered across files.

### B. Node Analysis Architecture

**experiments approach:**
```rust
// Trait-based with associated types
pub trait ParserTrait {
    type Checker: Alterator + Checker;
    type Getter: Getter;
    type Cognitive: Cognitive;
    // ... etc (12 associated types)
}
```

**cortex approach:**
```rust
// Unified analysis module with language dispatch
pub trait NodeChecker {
    fn is_comment(node: &Node, lang: Lang) -> bool;
    fn is_func(node: &Node, lang: Lang) -> bool;
    // ... etc (dispatch by Lang enum)
}

pub trait NodeGetter {
    fn get_func_space_name<'a>(node: &Node<'a>, code: &'a [u8], lang: Lang) -> Option<&'a str>;
    // ... etc (dispatch by Lang enum)
}
```

**Assessment:** cortex's approach is cleaner and more maintainable, but less type-safe. experiments is more type-safe but verbose.

### C. Concurrent Processing

**experiments:**
- `concurrent_files.rs` - Advanced concurrent file processing with thread pools
- Sophisticated channel-based producer-consumer pattern
- Job queue with error handling
- 150+ lines of optimized concurrent logic

**cortex:**
- `concurrent.rs` - Simplified concurrent processing
- Basic thread pool wrapper
- Less sophisticated error handling

**Migration Need:** cortex should adopt experiments' concurrent file processing patterns

### D. AST Handling

**experiments:**
- `ast.rs` - Custom AST structures with span information
- `alterator.rs` - AST mutation/transformation
- Bottom-up AST building algorithm

**cortex:**
- `ast_builder.rs` - Modern AST building with configuration
- `ast_editor.rs` - AST editing/manipulation with optimization
- More structured, better documented

**Assessment:** cortex's approach is more modern, but some functionality from experiments might be lost.

### E. Comment Removal

**experiments:**
- `comment_rm.rs` - Uses Aho-Corasick algorithm + regex

**cortex:**
- `comment_removal.rs` - More sophisticated with span tracking
- Returns CommentSpan information
- Better for code analysis requiring comment locations

### F. Output Formatting (Critical Gap)

**experiments:**
```
output/
├── dump.rs          - Generic dumping
├── dump_metrics.rs  - Metrics to JSON/YAML
├── dump_ops.rs      - Operations to JSON/YAML
└── mod.rs           - Output trait definitions
```

**cortex:**
- NO output formatting module
- **This is a critical gap** - no direct way to serialize metrics

### G. Type System & Traits

**experiments:**
- `traits.rs` (75 lines) - ParserTrait with associated types per metric
- `langs.rs` - Language enum and utilities
- Language-specific implementations scattered across language files

**cortex:**
- `traits.rs` (79 lines) - Cleaner design, language dispatch pattern
- `lang.rs` - Comprehensive Lang enum with more variants
- Centralized implementation in analysis/ module

---

## 5. Test Infrastructure

### experiments:
- `tests/deepspeech_test.rs` - Integration test against Mozilla DeepSpeech repo
- `tests/pdf_js_test.rs` - PDF.js project testing
- `tests/common/mod.rs` - Common test utilities with snapshot testing (insta)
- Test repositories included (serde, etc.)
- **Snapshot-based testing** for regression detection

### cortex:
- `src/analysis/tests.rs` - Unit tests for analysis module
- `src/lib.rs` - Module-level integration tests
- No complex integration test infrastructure
- No snapshot testing

**Gap:** cortex lacks comprehensive integration testing infrastructure

---

## 6. Advanced Features Missing from cortex

### 1. Preprocessor Support
- **experiments:** `preproc.rs` + `language_preproc.rs`
- **cortex:** `preprocessor.rs` exists but incomplete
- **Status:** Needs enhancement for C/C++ macro analysis

### 2. Spaces/Functions Analysis
- **experiments:** `spaces.rs` - detailed function space analysis
- **cortex:** `spaces.rs` - exists but may be simplified
- **Need to verify:** Functional equivalence

### 3. Operations Tracking
- **experiments:** `ops.rs` + special output module
- **cortex:** `ops.rs` exists but no output formatting
- **Gap:** Can't export operations data

### 4. Code Finding/Navigation
- **experiments:** `find.rs` - AST navigation utilities
- **cortex:** NOT PRESENT
- **Impact:** Loss of utility functions

### 5. Macro Support
- **experiments:** `macros.rs` + `c_langs_macros/`
- **cortex:** NO comprehensive macro support
- **Impact:** C/C++ analysis limited

---

## 7. Code Quality & Architecture Assessment

### cortex Improvements:
1. **Better documentation** - More comprehensive docstrings
2. **Cleaner module organization** - analysis/ module consolidates related functionality
3. **Modern design patterns** - Language dispatch via enums instead of associated types
4. **Enhanced types** - Dedicated types module with proper abstractions
5. **Tree-sitter integration** - Better wrapper with TextProvider trait
6. **Dependency extraction** - New sophisticated feature for code analysis

### experiments Strengths:
1. **Type safety** - Associated types provide compile-time guarantees
2. **Complete implementations** - More language variants (Kotlin, TSX, Mozilla JS)
3. **Output infrastructure** - Proper serialization support
4. **Testing** - Integration tests with snapshot validation
5. **Advanced features** - Macro support, preprocessor handling, space analysis
6. **Concurrent processing** - More sophisticated thread pool management

---

## 8. Specific Code Issues & Gaps

### A. Kotlin Support

**experiments:** Complete implementation
```
language_kotlin.rs - Full language support
```

**cortex:** 
```
src/languages/kotlin.rs - EXISTS but appears incomplete
```

**Action:** Verify kotlin.rs completeness; if incomplete, integrate from experiments

### B. TSX Support

**experiments:** Full TSX implementation (language_tsx.rs)

**cortex:** 
```
src/languages/tsx.rs - INCOMPLETE (needs migration)
```

**Action:** Complete TSX implementation from experiments/language_tsx.rs

### C. Mozilla JavaScript

**experiments:** 
```
language_mozjs.rs - Full Mozilla-specific JS support
```

**cortex:** NOT PRESENT

**Action:** Decide whether to include Mozilla JS support or document as unsupported

### D. Preprocessor Directives

**experiments:**
```
preproc.rs - C preprocessor handling
language_preproc.rs - Preprocessor-specific implementations
```

**cortex:**
```
preprocessor.rs - Incomplete implementation
```

**Action:** Enhance preprocessor.rs from experiments

### E. Output Formatting

**experiments:** Full output module (dump.rs, dump_metrics.rs, dump_ops.rs)

**cortex:** NO OUTPUT MODULE

**Critical Gap:** Cannot serialize analysis results to standard formats

---

## 9. Deprecated/Duplicate Code in cortex

### Identified Issues:

1. **Possible duplication:**
   - `function.rs` - exists in both (need to verify identical functionality)
   - `spaces.rs` - exists in both (need to verify identical functionality)
   - `comment_removal.rs` vs `comment_rm.rs` - similar names, verify duplication

2. **Incomplete implementations:**
   - `kotlin.rs` - exists but appears minimal
   - `tsx.rs` - exists but appears minimal
   - `preprocessor.rs` - incomplete compared to experiments

3. **API inconsistency:**
   - `detector_functions` might have changed signature
   - Some utilities missing (find.rs, count.rs, tools.rs)

---

## 10. Migration Roadmap

### Phase 1: Critical (Missing Language Features)
- [ ] Complete Kotlin implementation (copy from experiments or enhance cortex version)
- [ ] Complete TSX implementation (integrate from experiments)
- [ ] Verify function and spaces module parity

### Phase 2: High-Priority (Missing Functionality)
- [ ] Add output/ module for serialization support
- [ ] Complete preprocessor.rs implementation
- [ ] Add Mozilla JavaScript support (if needed)
- [ ] Implement find.rs and count.rs utilities

### Phase 3: Medium-Priority (Enhancement)
- [ ] Integrate advanced concurrent processing from experiments
- [ ] Add snapshot-based integration tests
- [ ] Enhance preprocessor macro support
- [ ] Add C-specific comment handling

### Phase 4: Documentation & Testing
- [ ] Create comprehensive integration tests
- [ ] Document language support limitations
- [ ] Add usage examples for each language
- [ ] Performance benchmarking

---

## 11. Architectural Improvements Needed

### 1. Trait Design Enhancement
**Current (cortex):**
```rust
pub trait NodeChecker {
    fn is_comment(node: &Node, lang: Lang) -> bool;
    fn is_func(node: &Node, lang: Lang) -> bool;
    // ... dispatches by Lang enum
}
```

**Suggested Improvement:**
Combine language-dispatch pattern with type safety using sealed traits or macros:
```rust
pub trait NodeChecker {
    fn is_comment(node: &Node) -> bool;
    fn is_func(node: &Node) -> bool;
    // Language-specific implementation via blanket impls or sealed trait
}

// Per-language implementations
impl NodeChecker for RustLanguage { ... }
impl NodeChecker for PythonLanguage { ... }
```

### 2. Output Serialization
**Need to add:**
```rust
pub trait MetricsSerializer {
    fn to_json(&self) -> String;
    fn to_yaml(&self) -> String;
    fn to_csv(&self) -> String;
}
```

### 3. Concurrent Processing
**Enhance cortex's concurrent.rs** with experiments' sophisticated patterns:
- Better error propagation
- Job queue with priority
- Progress tracking
- Resource limiting

### 4. Language Plugin System
Consider making languages pluggable rather than hardcoded:
```rust
pub trait LanguagePlugin {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &[&'static str];
    fn create_parser(&self) -> Result<Box<dyn Parser>>;
}
```

---

## 12. File-by-File Migration Status

### Ready to Migrate (from experiments → cortex):
- `output/` module (CRITICAL)
- `c_langs_macros/` module
- `language_mozjs.rs`
- `language_preproc.rs`
- `find.rs`
- `count.rs`
- `tools.rs`
- Enhanced `concurrent_files.rs` logic

### Need Verification (both have versions):
- `ast.rs` vs `ast_builder.rs`
- `alterator.rs` vs `ast_editor.rs`
- `comment_rm.rs` vs `comment_removal.rs`
- `function.rs` (both)
- `spaces.rs` (both)
- `traits.rs` (both)

### Need Completion in cortex:
- `languages/kotlin.rs`
- `languages/tsx.rs`
- `preprocessor.rs`
- `concurrent.rs` (enhance)

---

## Summary of Key Numbers

| Metric | experiments | cortex | Gap |
|--------|-------------|--------|-----|
| Source files | 52 | 47 | 5 fewer (output/, c_langs_macros/) |
| Languages | 11 (incl. MozJS, Preproc) | 8 | Missing 3 variants |
| Metric types | 12 | 12 | Equal |
| Module count | 20+ | 25+ | Cortex more modular |
| Output formats | JSON/YAML | NONE | Critical gap |
| Tests | Integration + Snapshot | Unit | Needs enhancement |
| Documentation | Minimal | Comprehensive | Cortex better |

---

## Recommendations

1. **Immediate:** Migrate output/ module (critical for usability)
2. **Short-term:** Complete Kotlin and TSX implementations
3. **Short-term:** Integrate Mozilla JS support
4. **Medium-term:** Enhance preprocessor support
5. **Medium-term:** Add utility modules (find, count, tools)
6. **Long-term:** Refactor to language plugin system
7. **Ongoing:** Maintain feature parity with comprehensive testing

---

**Report Generated:** 2025-10-25
**Analysis Scope:** Complete source code structure
**Recommended Action:** Review phase 1 critical items before next sprint

