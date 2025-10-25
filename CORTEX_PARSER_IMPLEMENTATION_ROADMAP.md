# Cortex-Parser Enhancement Roadmap Based on rust-code-analysis Analysis

## Phase 1: Foundation (Weeks 1-2)

### 1.1 Trait System Refactoring

**Goal**: Replace dynamic dispatch with compile-time checked metric trait bounds

```rust
// BEFORE (current approach)
pub trait Parser {
    fn parse(&self, code: &str) -> Result<AST>;
    fn get_metrics(&self) -> Metrics;
}

// AFTER (recommended)
pub trait LanguageInfo {
    type BaseLang;
    fn get_lang() -> Language;
}

pub trait ParserTrait
where
    Self: LanguageInfo 
        + Checker 
        + Cyclomatic 
        + Halstead 
        + Loc 
        + Mi
        + Cognitive
        + Abc
        + NomNom
        + NArgs
        + Exit
        + Wmc
{
    fn new(code: Vec<u8>, path: &Path) -> Self;
    fn get_root(&self) -> Node;
    fn get_code(&self) -> &[u8];
}

impl<T: ParserTrait> Parser<T> {
    pub fn parse(code: Vec<u8>, path: &Path) -> Self { ... }
    pub fn get_metrics(&self) -> CodeMetrics { ... }
}
```

**Impact**:
- Compile-time verification all metrics are implemented
- Zero-cost abstraction via monomorphization
- Enables generic metric computation
- Type-safe language dispatch

### 1.2 Core Module Structure

```rust
// src/traits.rs
pub trait LanguageInfo { ... }
pub trait Callback { type Res; type Cfg; ... }
pub trait Checker { is_comment, is_func, is_closure, ... }
pub trait Getter { get_func_name, get_space_kind, ... }

// src/node.rs
pub struct Node<'a>(OtherNode<'a>);
impl<'a> Node<'a> {
    pub fn count_specific_ancestors<T: ParserTrait>(...) -> usize
    pub fn has_ancestors(...) -> bool
}

// src/ast.rs
pub struct AstNode {
    pub r#type: &'static str,
    pub value: String,
    pub span: Span,  // (start_row, start_col, end_row, end_col)
    pub children: Vec<AstNode>,
}

// src/parser.rs
pub struct Parser<T: ParserTrait> {
    code: Vec<u8>,
    tree: Tree,
    phantom: PhantomData<T>,
}
```

**Deliverables**:
- Refactored traits module (~200 LOC)
- Node wrapper with lifetime safety (~150 LOC)
- Generic parser implementation (~100 LOC)
- Unit tests for each trait

### 1.3 Language Registration Infrastructure

```rust
// src/langs.rs
mk_langs!(
    (Rust, "Rust language", "rust", RustCode, RustParser,
     tree_sitter_rust, [rs], ["rust"]),
    (Cpp, "C/C++ language", "c/c++", CppCode, CppParser,
     tree_sitter_cpp, [cpp, c, h], ["c++", "c"]),
    // ... more languages
);
```

**Deliverables**:
- mk_langs! macro implementation (~150 LOC)
- Language registration system
- File extension mapping
- Runtime language dispatch

## Phase 2: Metrics Implementation (Weeks 3-4)

### 2.1 Foundation Metrics

Implement in dependency order:

**Lines of Code (LOC) - Week 3**
```rust
// src/metrics/loc.rs
pub struct Stats {
    sloc: f64,        // Source Lines
    ploc: f64,        // Physical Lines
    lloc: f64,        // Logical Lines
    cloc: f64,        // Comment Lines
    blank: usize,     // Blank Lines
}

impl Loc for Language {
    fn compute(node: &Node, stats: &mut Stats, is_func_space: bool, is_unit: bool) {
        // Count lines based on node span
        // Track min/max per space
    }
}
```

**Cyclomatic Complexity - Week 3**
```rust
// src/metrics/cyclomatic.rs
pub struct Stats {
    cyclomatic: f64,      // CC value
    cyclomatic_sum: f64,  // Aggregate
    cyclomatic_max: f64,  // Per-function max
    cyclomatic_min: f64,  // Per-function min
}

impl Cyclomatic for Language {
    fn compute(node: &Node, stats: &mut Stats) {
        // Count: if, switch, for, while, catch, &&, ||
        // Increment on each decision point
    }
}
```

**Halstead Metrics - Week 4**
```rust
// src/metrics/halstead.rs
pub struct HalsteadMaps<'a> {
    pub operators: HashMap<u16, u64>,       // Node kind -> count
    pub operands: HashMap<&'a [u8], u64>,   // Source bytes -> count
}

impl Halstead for Language {
    fn compute(node: &Node, code: &[u8], maps: &mut HalsteadMaps) {
        // Classify node as operator or operand
        // Update maps with counts
        // Calculate derived metrics: volume, effort, bugs, time
    }
}
```

**Deliverables**:
- LOC module (150 LOC, 80% language-independent)
- Cyclomatic module (200 LOC)
- Halstead module with maps (300 LOC)
- Test coverage for each metric

### 2.2 Derived Metrics

**Maintainability Index - Week 4**
```rust
// src/metrics/mi.rs
pub struct Stats {
    halstead_volume: f64,
    cyclomatic_sum: f64,
    sloc: f64,
    comments_percentage: f64,
}

impl Mi for Language {
    fn compute(loc: &loc::Stats, cc: &cyclomatic::Stats, h: &halstead::Stats, stats: &mut Stats) {
        // Original: 171 - 5.2*ln(V) - 0.23*CC - 16.2*ln(SLOC)
        // SEI: MI_orig + 50*sin(sqrt(2.4*CommentPct))
        // Visual Studio: (MI_orig * 100) / 171
    }
}
```

**Cognitive Complexity**
```rust
// src/metrics/cognitive.rs
pub struct Stats {
    structural: usize,
    nesting: usize,
    structural_sum: usize,
    structural_max: usize,
    structural_min: usize,
}

impl Cognitive for Language {
    fn compute(node: &Node, stats: &mut Stats, nesting_map: &mut HashMap<usize, (usize, usize, usize)>) {
        // Base: 1 point per decision
        // Nesting: Multiply by nesting depth
        // Else-if: Don't increment (special case)
    }
}
```

**ABC Metric**
```rust
// src/metrics/abc.rs
pub struct Stats {
    assignments: f64,    // A
    branches: f64,       // B (switch, ternary)
    conditions: f64,     // C (if, while, for)
}
// Magnitude: sqrt(A^2 + B^2 + C^2)
```

**Deliverables**:
- MI module with 3 formulas (150 LOC)
- Cognitive complexity (200 LOC)
- ABC metric (150 LOC)
- Integration tests with known data

### 2.3 Object-Oriented Metrics

**NOM (Number of Methods), NArgs, NExits, WMC, NPM, NPA**
```rust
// src/metrics/nom.rs
pub struct Stats {
    functions: usize,
    closures: usize,
    functions_sum: usize,
    closures_sum: usize,
}

// src/metrics/nargs.rs
pub struct Stats {
    fn_nargs: usize,
    closure_nargs: usize,
    // ... aggregate tracking
}

// ... similar for exit, wmc, npm, npa
```

**Deliverables**:
- 6 OO metric modules (~100 LOC each)
- Space-aware computation
- Per-scope tracking

## Phase 3: Multi-Language Support (Weeks 5-6)

### 3.1 Language Infrastructure

**Language Enum Generation**
```rust
// src/languages/language_rust.rs
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum Rust {
    End = 0,
    Identifier = 1,
    // ... 300+ node kind variants
}

// src/languages/language_cpp.rs
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum Cpp {
    End = 0,
    Identifier = 1,
    // ... 500+ node kind variants
}
```

**Per-Language Implementations**
```rust
// src/languages/checker_rust.rs
impl Checker for RustCode {
    fn is_comment(node: &Node) -> bool {
        matches!(node.kind_id().into(), Rust::LineComment | Rust::BlockComment)
    }
    
    fn is_func_space(node: &Node) -> bool {
        matches!(node.kind_id().into(), 
            Rust::FunctionItem | Rust::ClosureExpression | Rust::TraitItem)
    }
}

// src/languages/getter_rust.rs
impl Getter for RustCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        match node.kind_id().into() {
            Rust::FunctionItem => SpaceKind::Function,
            Rust::StructItem => SpaceKind::Struct,
            Rust::TraitItem => SpaceKind::Trait,
            // ...
        }
    }
}

// src/languages/metrics_rust.rs
impl Cyclomatic for RustCode {
    fn compute(node: &Node, stats: &mut Stats) { ... }
}
impl Halstead for RustCode { ... }
// ... per-metric implementation
```

**Deliverables**:
- Language enum generators (per-language: ~500 LOC)
- Checker implementations (per-language: ~200 LOC)
- Getter implementations (per-language: ~300 LOC)
- Metric trait overrides (per-language: ~500 LOC)

### 3.2 Languages to Prioritize (in order)

**Tier 1 - Week 5:**
1. Rust (simplest, already have tree-sitter grammar)
2. Python (indentation-based, moderate complexity)

**Tier 2 - Week 6:**
3. JavaScript/TypeScript (ambiguous syntax, function detection challenge)
4. Java (class-heavy, standard structure)

**Estimated per-language: 1500-2000 LOC**

## Phase 4: Advanced Features (Weeks 7-8)

### 4.1 Concurrent File Processing

```rust
// src/concurrent_files.rs
pub struct FilesData {
    pub paths: Vec<PathBuf>,
    pub include: GlobSet,
    pub exclude: GlobSet,
}

fn consumer<Config, Proc>(receiver: Receiver<Option<JobItem<Config>>>, func: Arc<Proc>)
where
    Proc: Fn(PathBuf, &Config) -> io::Result<()> + Send + Sync
{
    while let Ok(Some(job)) = receiver.recv() {
        if let Err(e) = func(job.path.clone(), &job.cfg) {
            eprintln!("Error processing {:?}: {:?}", job.path, e);
        }
    }
}

pub fn process_files<Config, Proc>(
    paths: Vec<PathBuf>,
    include: Vec<String>,
    exclude: Vec<String>,
    config: Arc<Config>,
    processor: Arc<Proc>,
) -> Result<(), ConcurrentErrors>
where
    Proc: Fn(PathBuf, &Config) -> io::Result<()> + Send + Sync + 'static,
{
    let (tx, rx) = unbounded();
    let num_workers = num_cpus::get();
    
    let handles = (0..num_workers)
        .map(|_| {
            let rx = rx.clone();
            let processor = processor.clone();
            thread::spawn(move || consumer(rx, processor))
        })
        .collect::<Vec<_>>();
    
    // Send jobs from walker
    for path in walker.paths {
        tx.send(Some(JobItem { path, cfg: config.clone() }))?;
    }
    
    // Signal completion
    for _ in 0..num_workers {
        tx.send(None)?;
    }
}
```

**Deliverables**:
- concurrent_files module (250 LOC)
- Job distribution system
- Error aggregation
- Performance testing

### 4.2 C/C++ Preprocessor Integration

```rust
// src/preproc.rs
pub struct PreprocFile {
    pub direct_includes: HashSet<String>,
    pub indirect_includes: HashSet<String>,
    pub macros: HashSet<String>,
}

pub fn fix_includes(
    files: &mut HashMap<PathBuf, PreprocFile>,
    all_files: &HashMap<String, Vec<PathBuf>>,
) {
    let mut g = StableGraph::new();
    let mut nodes: HashMap<PathBuf, NodeIndex> = HashMap::new();
    
    // Build graph
    for (file, pf) in files.iter() {
        let node = nodes.entry(file.clone())
            .or_insert_with(|| g.add_node(file.clone()));
        
        for include in &pf.direct_includes {
            let possibilities = guess_file(file, include, all_files);
            for included_file in possibilities {
                if included_file != *file {
                    let inc_node = nodes.entry(included_file.clone())
                        .or_insert_with(|| g.add_node(included_file));
                    g.add_edge(*node, *inc_node, 0);
                }
            }
        }
    }
    
    // Detect SCCs (circular includes)
    let sccs = kosaraju_scc(&g);
    
    // Propagate macros through includes
    for scc in sccs {
        let mut all_macros = HashSet::new();
        for node in scc {
            if let Some(file) = g.node_weight(node) {
                if let Some(pf) = files.get(file) {
                    all_macros.extend(pf.macros.iter().cloned());
                }
            }
        }
        for node in scc {
            if let Some(file) = g.node_weight(node) {
                if let Some(pf) = files.get_mut(file) {
                    pf.macros.extend(all_macros.iter().cloned());
                }
            }
        }
    }
}
```

**Deliverables**:
- Preprocessor handling (400 LOC)
- Include graph construction
- SCC detection
- Macro resolution

### 4.3 AST Serialization and Output

```rust
// src/output/dump.rs
pub fn dump_node(code: &[u8], node: &Node, depth: usize) -> io::Result<()> {
    let indent = "  ".repeat(depth);
    println!("{}{}{}",
        indent,
        node.kind(),
        if let Some(text) = node.utf8_text(code) {
            format!(" '{}'", text)
        } else {
            String::new()
        }
    );
}

// src/output/metrics.rs
#[derive(Serialize)]
pub struct CodeMetrics {
    pub cyclomatic: cyclomatic::Stats,
    pub halstead: halstead::Stats,
    pub loc: loc::Stats,
    pub mi: mi::Stats,
    pub cognitive: cognitive::Stats,
    pub abc: abc::Stats,
    pub nom: nom::Stats,
    pub nargs: nargs::Stats,
    pub nexits: exit::Stats,
    pub wmc: wmc::Stats,
    pub npm: npm::Stats,
    pub npa: npa::Stats,
}

// src/output/mod.rs
pub fn export_metrics_json(metrics: &CodeMetrics) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(metrics)
}

pub fn export_metrics_yaml(metrics: &CodeMetrics) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(metrics)
}
```

**Deliverables**:
- dump module (150 LOC)
- metrics serialization (100 LOC)
- JSON/YAML export
- Tree visualization

## Phase 5: Testing & Validation (Week 9)

### 5.1 Snapshot Testing with Insta

```rust
#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    
    #[test]
    fn test_rust_metrics() {
        let code = "fn foo(x: i32) -> i32 { if x > 0 { x * 2 } else { 0 } }";
        let parser = RustParser::new(code.as_bytes().to_vec(), Path::new("test.rs"), None);
        let metrics = parser.get_metrics();
        
        assert_snapshot!(metrics);
    }
    
    #[test]
    fn test_cpp_preprocessor() {
        let code = "#include <stdio.h>\n#define MAX 100\nint main() { return 0; }";
        let parser = CppParser::new(code.as_bytes().to_vec(), Path::new("test.c"), None);
        let metrics = parser.get_metrics();
        
        assert_snapshot!(metrics);
    }
}
```

**Deliverables**:
- Unit tests for each metric
- Integration tests per language
- Snapshot references
- Real-world codebase testing

### 5.2 Performance Testing

```rust
#[bench]
fn bench_parse_large_file(b: &mut Bencher) {
    let code = include_bytes!("../test-data/large_file.rs");
    b.iter(|| {
        let _parser = RustParser::new(code.to_vec(), Path::new("large.rs"), None);
    });
}

#[bench]
fn bench_concurrent_processing(b: &mut Bencher) {
    let paths = vec!["test1.rs", "test2.rs", ...].into_iter().map(PathBuf::from).collect();
    b.iter(|| {
        process_files(paths.clone(), vec![], vec![], Arc::new(config), Arc::new(processor))
    });
}
```

**Deliverables**:
- Benchmark suite
- Performance regression detection
- Memory profiling
- Scalability validation

## Phase 6: Documentation & Polish (Week 10)

### 6.1 API Documentation

- Module-level rustdoc comments
- Type-level trait documentation
- Usage examples per metric
- Architecture diagrams

### 6.2 Integration Testing

- Test against real-world projects
- Cross-language comparison
- Metrics validation
- Performance profiling

### 6.3 Release Preparation

- CHANGELOG updates
- Version management
- CI/CD setup
- Coverage reports

## Implementation Checklist

### Core Infrastructure
- [ ] Refactor traits system (ParserTrait pattern)
- [ ] Implement language registration macros
- [ ] Create generic parser framework
- [ ] Add node wrapper with lifetime safety

### Metrics (Phase 2)
- [ ] Implement LOC variants (SLOC, PLOC, LLOC, CLOC)
- [ ] Add Cyclomatic Complexity
- [ ] Implement Halstead Metrics suite
- [ ] Add Maintainability Index (3 formulas)
- [ ] Cognitive Complexity
- [ ] ABC Metric
- [ ] NOM, NArgs, NExits, WMC, NPM, NPA

### Languages (Phase 3)
- [ ] Rust support
- [ ] Python support
- [ ] JavaScript/TypeScript support
- [ ] Java support

### Advanced Features (Phase 4)
- [ ] Concurrent file processing
- [ ] C/C++ preprocessor handling
- [ ] AST serialization
- [ ] Metrics export (JSON/YAML)

### Testing (Phase 5)
- [ ] Unit test coverage
- [ ] Integration tests
- [ ] Snapshot testing
- [ ] Performance benchmarks

### Documentation (Phase 6)
- [ ] API documentation
- [ ] Architecture guides
- [ ] Example usage
- [ ] Troubleshooting guide

## Dependencies Required

```toml
[dependencies]
tree-sitter = "0.25.3"
tree-sitter-rust = "0.23.2"
tree-sitter-python = "0.23.6"
tree-sitter-javascript = "0.23.1"
tree-sitter-typescript = "0.23.2"
tree-sitter-java = "0.23.5"

serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

crossbeam = { version = "0.8", features = ["crossbeam-channel"] }
walkdir = "2.3"
globset = "0.4"
petgraph = "0.8"

regex = "1.7"
aho-corasick = "1.0"
termcolor = "1.2"
num-format = "0.4"

[dev-dependencies]
insta = { version = "1.29.0", features = ["yaml", "json"] }
criterion = "0.5"
```

## Success Criteria

- All 12 metrics implemented across all target languages
- Concurrent processing 4x faster than sequential
- Handles 1000+ file projects in < 5 seconds
- Test coverage > 80%
- Zero unsafe code
- All metrics validated against known datasets
- API documentation complete
- 0 panics in production use

## Timeline

- **Phase 1 (Weeks 1-2)**: Foundation - 25 LOC per person-hour
- **Phase 2 (Weeks 3-4)**: Metrics - 20 LOC per person-hour
- **Phase 3 (Weeks 5-6)**: Languages - 30 LOC per person-hour
- **Phase 4 (Weeks 7-8)**: Features - 25 LOC per person-hour
- **Phase 5 (Week 9)**: Testing - 50 tests per person-hour
- **Phase 6 (Week 10)**: Polish - Documentation complete

**Total Estimated Effort**: 2-3 senior engineers for 10 weeks (~15,000 LOC production + 5,000 LOC tests)
