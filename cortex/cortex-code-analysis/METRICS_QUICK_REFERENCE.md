# Cortex Code Analysis - Metrics Quick Reference

## Available Metrics

### Complexity Metrics

#### Cyclomatic Complexity
Measures the number of linearly independent paths through code.
```rust
use cortex_code_analysis::metrics::CyclomaticStats;

let stats = CyclomaticStats::new();
// stats.cyclomatic_sum()    - Total cyclomatic complexity
// stats.cyclomatic_average() - Average complexity per function
// stats.cyclomatic_min()     - Minimum complexity
// stats.cyclomatic_max()     - Maximum complexity
```

#### Cognitive Complexity
Measures how difficult code is to understand.
```rust
use cortex_code_analysis::metrics::CognitiveStats;

let mut stats = CognitiveStats::new();
stats.increment_with_nesting(2);  // +3 (1 + 2 nesting levels)
stats.eval_boolean_sequence(and_id);  // Smart boolean counting
```

**Key Features**:
- Sequential boolean operators: `a && b && c` counts as +1
- Different operators: `a && b || c` counts as +2
- Nesting penalties automatically applied

### Size Metrics

#### Lines of Code (LOC)
Counts various types of lines.
```rust
use cortex_code_analysis::metrics::LocStats;

let stats = LocStats::new();
// stats.sloc()  - Source lines of code
// stats.ploc()  - Physical lines
// stats.lloc()  - Logical lines
// stats.cloc()  - Comment lines
// stats.blank() - Blank lines
```

#### Halstead Metrics
Measures program vocabulary and volume.
```rust
use cortex_code_analysis::metrics::{HalsteadStats, HalsteadMaps, HalsteadCollector};

// Method 1: Using HalsteadMaps (AST-level)
let mut maps = HalsteadMaps::new();
*maps.operators.entry(kind_id).or_insert(0) += 1;
*maps.operands.entry(b"variable").or_insert(0) += 1;
let top_ops = maps.most_frequent_operators(10);

// Method 2: Using HalsteadCollector (high-level)
let mut collector = HalsteadCollector::new();
collector.add_operator("+");
collector.add_operand("x");
let stats = collector.finalize();

// Metrics available:
// stats.u_operators()  - Unique operators (η1)
// stats.operators()    - Total operators (N1)
// stats.u_operands()   - Unique operands (η2)
// stats.operands()     - Total operands (N2)
// stats.length()       - Program length (N)
// stats.vocabulary()   - Program vocabulary (η)
// stats.volume()       - Program volume (V)
// stats.difficulty()   - Difficulty (D)
// stats.effort()       - Effort (E)
// stats.time()         - Time to program (seconds)
// stats.bugs()         - Estimated bugs (B)
```

### Design Metrics

#### ABC (Assignments, Branches, Conditions)
Measures code size through counting.
```rust
use cortex_code_analysis::metrics::AbcStats;

let mut stats = AbcStats::new();

// Simple counting
stats.add_assignment();
stats.add_branch();
stats.add_condition();

// Advanced: Declaration context (distinguishes var from const)
stats.start_var_declaration();
stats.add_assignment_with_context();  // Counts as assignment
stats.clear_declaration();

stats.start_const_declaration();
stats.add_assignment_with_context();  // Does NOT count
stats.clear_declaration();

// Java final support
stats.start_var_declaration();
stats.promote_to_const();  // final int x = 5;
stats.add_assignment_with_context();  // Does NOT count
stats.clear_declaration();

// Get results
// stats.assignments()       - Assignment count
// stats.branches()          - Branch count
// stats.conditions()        - Condition count
// stats.magnitude()         - sqrt(A² + B² + C²)
// stats.magnitude_sum()     - Total magnitude
```

#### Number of Methods (NOM)
Counts functions and closures.
```rust
use cortex_code_analysis::metrics::NomStats;

let stats = NomStats::new();
// stats.functions_sum()     - Total functions
// stats.closures_sum()      - Total closures
// stats.total()             - Functions + closures
// stats.functions_average() - Average functions per space
// stats.functions_min/max() - Min/max functions
```

#### Weighted Methods per Class (WMC)
Sums complexity of all methods in a class.
```rust
use cortex_code_analysis::metrics::WmcStats;

let stats = WmcStats::from_cyclomatic(&cyclomatic_stats);
// stats.wmc() - Total weighted complexity
```

#### Number of Public Methods/Attributes (NPM/NPA)
Counts public members in classes.
```rust
use cortex_code_analysis::metrics::{NpmStats, NpaStats};

let npm = NpmStats::new();
let npa = NpaStats::new();
```

### Maintainability Metrics

#### Maintainability Index (MI)
Composite metric for code maintainability.
```rust
use cortex_code_analysis::metrics::MaintainabilityIndexStats;

let mi = MaintainabilityIndexStats::from_metrics(&loc, &cyclomatic, &halstead);
// mi.mi_original()      - Original formula
// mi.mi_sei()           - SEI formula (with comments)
// mi.mi_visual_studio() - Visual Studio formula (0-100)
```

### Other Metrics

#### Exit Points
Counts possible exit points from functions.
```rust
use cortex_code_analysis::metrics::ExitStats;

let mut stats = ExitStats::new();
stats.increment();  // Found return/exit
// stats.exit_sum()     - Total exits
// stats.exit_average() - Average per function
// stats.exit_min/max() - Min/max exits
```

#### Number of Arguments (NArgs)
Counts function/method parameters.
```rust
use cortex_code_analysis::metrics::NargsStats;

let stats = NargsStats::new();
// stats.nargs_sum()     - Total arguments
// stats.nargs_average() - Average per function
// stats.nargs_min/max() - Min/max arguments
```

## Node Analysis Traits

### NodeChecker
Determines node properties.
```rust
use cortex_code_analysis::analysis::checker::{NodeChecker, DefaultNodeChecker};
use cortex_code_analysis::Lang;

// Check if node is a comment
if DefaultNodeChecker::is_comment(&node, Lang::Rust) { }

// Check if node is a function
if DefaultNodeChecker::is_func(&node, Lang::Python) { }

// Check if node is a closure
if DefaultNodeChecker::is_closure(&node, Lang::JavaScript) { }

// Advanced: Count specific ancestors
let count = node.count_specific_ancestors(
    |n| n.kind() == "function_declaration",  // Looking for
    |n| n.kind() == "class_declaration"      // Stop at
);
```

### NodeGetter
Extracts information from nodes.
```rust
use cortex_code_analysis::analysis::getter::{NodeGetter, DefaultNodeGetter};

// Get function name
let name = DefaultNodeGetter::get_func_name(&node, code, Lang::Rust);

// Get space kind
let kind = DefaultNodeGetter::get_space_kind(&node, Lang::Java);

// Get Halstead operator/operand type
let op_type = DefaultNodeGetter::get_op_type(&node, Lang::TypeScript);
```

## Complete Metrics Suite

Use `CodeMetrics` for all metrics at once:
```rust
use cortex_code_analysis::metrics::CodeMetrics;

let mut metrics = CodeMetrics::new();
// ... compute metrics ...
metrics.compute_derived();  // Computes MI and WMC

// Access all metrics
println!("Cyclomatic: {}", metrics.cyclomatic);
println!("LOC: {}", metrics.loc);
println!("Halstead: {}", metrics.halstead);
println!("ABC: {}", metrics.abc);
println!("Cognitive: {}", metrics.cognitive);
println!("MI: {}", metrics.maintainability_index);
println!("Exit: {}", metrics.exit);
println!("NOM: {}", metrics.nom);
println!("NArgs: {}", metrics.nargs);
println!("NPM: {}", metrics.npm);
println!("NPA: {}", metrics.npa);
println!("WMC: {}", metrics.wmc);

// Merge metrics from multiple files
metrics.merge(&other_metrics);
```

## Language Support

All metrics support:
- ✅ Rust
- ✅ Python
- ✅ JavaScript
- ✅ TypeScript
- ✅ TSX
- ✅ Java
- ✅ C++
- ⚠️ Kotlin (partial)

## Common Patterns

### Computing Metrics Per Function
```rust
// 1. Create stats struct
let mut stats = CognitiveStats::new();

// 2. Traverse AST and compute
for node in ast.walk() {
    // Language-specific computation
    // stats.increment(), etc.
}

// 3. Finalize (if needed)
stats.compute_minmax();
```

### Merging Metrics from Multiple Files
```rust
let mut total = CodeMetrics::new();

for file in files {
    let file_metrics = compute_metrics(file);
    total.merge(&file_metrics);
}
```

### Getting Min/Max Values
```rust
// Most metrics provide min/max tracking
let min_complexity = cognitive.cognitive_min();
let max_complexity = cognitive.cognitive_max();
let avg_complexity = cognitive.cognitive_average();
```

## Best Practices

1. **Use Derived Metrics**: Call `compute_derived()` on `CodeMetrics` to get MI and WMC
2. **Clear Contexts**: Always call `clear_declaration()` after declaration blocks (ABC)
3. **Reset Sequences**: Call `reset_boolean_seq()` when starting new expressions (Cognitive)
4. **Check Min Values**: Min values may be `usize::MAX` or `f64::MAX` if never set - use getters that handle this
5. **Finalize Stats**: Call `compute_minmax()` before reading sum/min/max values

## Advanced Features

### HalsteadMaps Frequency Analysis
```rust
let mut maps = HalsteadMaps::new();
// ... populate maps ...

// Get top 10 operators
let top_ops = maps.most_frequent_operators(10);
for (kind_id, count) in top_ops {
    println!("Operator {} used {} times", kind_id, count);
}

// Get top 10 operands
let top_operands = maps.most_frequent_operands(10);
for (operand, count) in top_operands {
    println!("Operand {:?} used {} times", operand, count);
}
```

### ABC Declaration Lifecycle
```rust
// For Java: final int x = 5, y = 6;
stats.start_var_declaration();       // Start of field_declaration
stats.promote_to_const();             // Encountered "final"
stats.add_assignment_with_context();  // x = 5 (NOT counted)
stats.add_assignment_with_context();  // y = 6 (NOT counted)
stats.clear_declaration();            // End of statement

// For JavaScript: let x = 1; const y = 2;
stats.start_var_declaration();        // let declaration
stats.add_assignment_with_context();  // x = 1 (counted)
stats.clear_declaration();

stats.start_const_declaration();      // const declaration
stats.add_assignment_with_context();  // y = 2 (NOT counted)
stats.clear_declaration();
```

### Cognitive Complexity Boolean Sequences
```rust
// For: if (a && b && c || d) { ... }
stats.increment();                    // if statement: +1
stats.eval_boolean_sequence(and_id);  // first &&: +1 (total: 2)
stats.eval_boolean_sequence(and_id);  // second && (same): +0 (total: 2)
stats.eval_boolean_sequence(or_id);   // || (different): +1 (total: 3)
```

## Documentation

For detailed information:
- **Migration Report**: See `ADVANCED_METRICS_MIGRATION.md`
- **Implementation Guide**: See `MIGRATION_SUMMARY.md`
- **API Docs**: Run `cargo doc --open` in the cortex-code-analysis directory
- **Tests**: See `src/metrics/*/tests` modules for usage examples

---

**Quick Tip**: All stats structs implement `Display`, so you can easily print them:
```rust
println!("{}", cognitive_stats);
println!("{}", halstead_stats);
println!("{}", abc_stats);
```
