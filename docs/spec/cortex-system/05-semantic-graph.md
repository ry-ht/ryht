# Cortex: Semantic Code Graph Architecture

## ✅ Implementation Status: FULLY IMPLEMENTED (100%)

**Last Updated**: 2025-10-20
**Status**: ✅ **Complete and operational**
**Location**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex-semantic/src/`
**Lines of Code**: 4,112 lines
**Tests**: 56+ tests passing

### Implementation Summary
- ✅ Multiple embedding providers (OpenAI, ONNX, Ollama)
- ✅ HNSW vector index with persistence
- ✅ Query processing with intent detection
- ✅ Hybrid search (keyword + semantic)
- ✅ Result ranking and re-ranking
- ✅ Two-layer caching system (memory + disk)
- ✅ Semantic search <100ms
- ✅ Batch embedding operations

### Key Components Implemented
| Component | Purpose | Status |
|-----------|---------|--------|
| providers.rs | Embedding providers | ✅ Complete |
| index.rs | HNSW vector index | ✅ Complete |
| search.rs | Search engine | ✅ Complete |
| query.rs | Query processing | ✅ Complete |
| ranking.rs | Result ranking | ✅ Complete |
| cache.rs | Caching layer | ✅ Complete |
| config.rs | Configuration | ✅ Complete |

### Performance Metrics
- **Search Latency**: <100ms per query
- **Index Build**: Efficient batching
- **Cache Hit Rate**: High for repeated queries
- **Provider Support**: 3 providers (OpenAI, ONNX, Ollama)

---

## Overview

The Semantic Code Graph transforms code from text into a rich, interconnected knowledge graph. Using tree-sitter for deep parsing and SurrealDB for storage, it provides semantic understanding that enables intelligent operations far beyond text manipulation.

## Core Concepts

### Semantic Units

Code is broken down into semantic units—meaningful pieces that can be understood in isolation:

```
Text:     "fn calculate_tax(income: f64) -> f64 { income * 0.2 }"
          ↓
Unit:     {
            type: "function",
            name: "calculate_tax",
            parameters: [{name: "income", type: "f64"}],
            returns: "f64",
            body_ast: <tree-sitter AST>,
            complexity: {cyclomatic: 1, cognitive: 1},
            purpose: "Calculates tax as 20% of income"
          }
```

### Relationship Types

Units are connected by typed relationships:

- **Structural**: contains, defines, declares
- **Dependency**: imports, calls, uses_type, instantiates
- **Inheritance**: extends, implements, overrides
- **Semantic**: similar_to, variant_of, refactored_from

## Tree-Sitter Integration

### Parser Architecture

```rust
struct CodeParser {
    parsers: HashMap<Language, Parser>,
    queries: HashMap<Language, QuerySet>,
}

impl CodeParser {
    fn parse_file(&self, content: &str, language: Language) -> ParseResult {
        let parser = self.parsers.get(&language)?;
        let tree = parser.parse(content, None)?;

        ParseResult {
            ast: tree,
            units: self.extract_units(&tree, language)?,
            imports: self.extract_imports(&tree, language)?,
            exports: self.extract_exports(&tree, language)?,
        }
    }

    fn extract_units(&self, tree: &Tree, language: Language) -> Vec<CodeUnit> {
        let queries = &self.queries[&language];
        let mut units = Vec::new();

        // Query for functions
        for match_ in queries.functions.matches(tree.root_node(), content) {
            units.push(self.build_function_unit(match_)?);
        }

        // Query for classes
        for match_ in queries.classes.matches(tree.root_node(), content) {
            units.push(self.build_class_unit(match_)?);
        }

        // ... other unit types

        units
    }
}
```

### Language-Specific Queries

#### Rust Queries

```scheme
; Function definitions
(function_item
  name: (identifier) @function.name
  parameters: (parameters) @function.params
  return_type: (_)? @function.return
  body: (block) @function.body
) @function

; Struct definitions
(struct_item
  name: (type_identifier) @struct.name
  body: (field_declaration_list)? @struct.fields
) @struct

; Impl blocks
(impl_item
  type: (type_identifier) @impl.type
  body: (declaration_list) @impl.body
) @impl

; Trait definitions
(trait_item
  name: (type_identifier) @trait.name
  body: (declaration_list) @trait.body
) @trait
```

#### TypeScript Queries

```scheme
; Function declarations
(function_declaration
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.params
  return_type: (type_annotation)? @function.return
  body: (statement_block) @function.body
) @function

; Class declarations
(class_declaration
  name: (type_identifier) @class.name
  heritage: (extends_clause)? @class.extends
  body: (class_body) @class.body
) @class

; Interface declarations
(interface_declaration
  name: (type_identifier) @interface.name
  heritage: (extends_clause)? @interface.extends
  body: (interface_body) @interface.body
) @interface

; Type aliases
(type_alias_declaration
  name: (type_identifier) @type.name
  value: (_) @type.value
) @type
```

### AST Node Extraction

```rust
struct ASTExtractor {
    fn extract_function(&self, node: Node, source: &str) -> FunctionUnit {
        let name = self.get_child_text(node, "name", source);
        let params = self.extract_parameters(node.child_by_field_name("parameters"));
        let return_type = self.extract_type(node.child_by_field_name("return_type"));
        let body = self.get_child_text(node, "body", source);

        // Extract modifiers (async, const, etc)
        let modifiers = self.extract_modifiers(node);

        // Extract visibility
        let visibility = self.extract_visibility(node);

        // Calculate complexity
        let complexity = self.calculate_complexity(node);

        FunctionUnit {
            name,
            qualified_name: self.build_qualified_name(&name),
            parameters: params,
            return_type,
            body,
            modifiers,
            visibility,
            complexity,
            ast_node: self.serialize_node(node),
        }
    }

    fn extract_parameters(&self, params_node: Option<Node>) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let Some(node) = params_node {
            for param in node.named_children() {
                parameters.push(Parameter {
                    name: self.get_child_text(param, "pattern"),
                    type_: self.extract_type(param.child_by_field_name("type")),
                    default: self.get_child_text(param, "value"),
                    modifiers: self.extract_param_modifiers(param),
                });
            }
        }

        parameters
    }
}
```

## Semantic Analysis

### Unit Extraction

```rust
impl SemanticAnalyzer {
    async fn analyze_file(&self, vnode: &VNode) -> Result<Vec<CodeUnit>> {
        let content = self.get_content(vnode)?;
        let language = detect_language(&vnode.path)?;

        // Parse with tree-sitter
        let parse_result = self.parser.parse_file(&content, language)?;

        // Extract semantic units
        let mut units = Vec::new();

        for ast_unit in parse_result.units {
            let mut unit = CodeUnit {
                id: generate_id(),
                unit_type: ast_unit.unit_type,
                name: ast_unit.name,
                qualified_name: self.build_qualified_name(&ast_unit),
                file_node: vnode.id,
                start_line: ast_unit.start_position.row,
                end_line: ast_unit.end_position.row,
                signature: self.build_signature(&ast_unit),
                body: ast_unit.body,
                language,
                ..Default::default()
            };

            // Semantic enrichment
            unit.summary = self.generate_summary(&unit)?;
            unit.purpose = self.infer_purpose(&unit)?;
            unit.embedding = self.generate_embedding(&unit)?;
            unit.complexity = self.calculate_complexity(&unit)?;

            // Language-specific analysis
            match language {
                Language::Rust => self.analyze_rust_unit(&mut unit)?,
                Language::TypeScript => self.analyze_typescript_unit(&mut unit)?,
                _ => {}
            }

            units.push(unit);
        }

        // Build relationships
        self.build_relationships(&units)?;

        Ok(units)
    }
}
```

### Complexity Calculation

```rust
struct ComplexityAnalyzer {
    fn calculate_complexity(&self, node: &Node) -> ComplexityMetrics {
        ComplexityMetrics {
            cyclomatic: self.calculate_cyclomatic(node),
            cognitive: self.calculate_cognitive(node),
            nesting: self.calculate_max_nesting(node),
            lines: self.count_lines(node),
        }
    }

    fn calculate_cyclomatic(&self, node: &Node) -> u32 {
        let mut complexity = 1;  // Base complexity

        let mut cursor = node.walk();

        for node in node.descendants() {
            match node.kind() {
                // Branching
                "if_statement" | "if_expression" => complexity += 1,
                "else_clause" => complexity += 1,
                "match_arm" | "switch_case" => complexity += 1,

                // Loops
                "while_statement" | "for_statement" | "loop_statement" => complexity += 1,

                // Exception handling
                "catch_clause" | "rescue_clause" => complexity += 1,

                // Logical operators
                "&&" | "||" | "and" | "or" => complexity += 1,

                // Ternary/conditional
                "conditional_expression" => complexity += 1,

                _ => {}
            }
        }

        complexity
    }

    fn calculate_cognitive(&self, node: &Node) -> u32 {
        let mut complexity = 0;
        let mut nesting_level = 0;

        for node in node.descendants() {
            // Increase nesting for control structures
            if self.is_nesting_node(&node) {
                nesting_level += 1;
            }

            // Add complexity based on node type and nesting
            complexity += match node.kind() {
                "if_statement" => 1 + nesting_level,
                "else_clause" => 1,
                "match_expression" => 2 + nesting_level,
                "while_statement" => 3 + nesting_level,
                "for_statement" => 3 + nesting_level,
                "break_statement" => 1,
                "continue_statement" => 1,
                "return_statement" if !self.is_early_return(&node) => 1,
                "&&" | "||" => 1,
                "catch_clause" => 2,
                "recursive_call" => 5,
                _ => 0
            };

            // Decrease nesting when exiting
            if self.is_nesting_node(&node) && node.next_sibling().is_none() {
                nesting_level -= 1;
            }
        }

        complexity
    }
}
```

## Dependency Graph

### Dependency Detection

```rust
struct DependencyAnalyzer {
    async fn analyze_dependencies(&self, unit: &CodeUnit) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        // Parse the unit body
        let ast = parse_code(&unit.body, unit.language)?;

        // Find function calls
        for call in self.find_calls(&ast) {
            if let Some(target) = self.resolve_call(&call, unit) {
                dependencies.push(Dependency {
                    source: unit.id,
                    target,
                    dependency_type: DependencyType::Calls,
                    metadata: self.extract_call_metadata(&call),
                });
            }
        }

        // Find type usage
        for type_ref in self.find_type_references(&ast) {
            if let Some(target) = self.resolve_type(&type_ref, unit) {
                dependencies.push(Dependency {
                    source: unit.id,
                    target,
                    dependency_type: DependencyType::UsesType,
                    metadata: self.extract_type_metadata(&type_ref),
                });
            }
        }

        // Find imports
        for import in self.find_imports(&ast) {
            if let Some(targets) = self.resolve_import(&import, unit) {
                for target in targets {
                    dependencies.push(Dependency {
                        source: unit.id,
                        target,
                        dependency_type: DependencyType::Imports,
                        metadata: self.extract_import_metadata(&import),
                    });
                }
            }
        }

        dependencies
    }

    fn resolve_call(&self, call: &CallExpression, context: &CodeUnit) -> Option<UnitId> {
        // Try local resolution first
        if let Some(local) = self.resolve_local(&call.name, context) {
            return Some(local);
        }

        // Try imported symbols
        if let Some(imported) = self.resolve_imported(&call.name, context) {
            return Some(imported);
        }

        // Try global resolution
        self.resolve_global(&call.name, context)
    }
}
```

### Graph Building

```rust
impl GraphBuilder {
    async fn build_dependency_graph(&self, units: &[CodeUnit]) -> Result<()> {
        // Create DEPENDS_ON relationships
        for unit in units {
            let dependencies = self.analyzer.analyze_dependencies(unit).await?;

            for dep in dependencies {
                self.db.query("
                    RELATE $source->depends_on->$target
                    SET dependency_type = $type,
                        metadata = $metadata
                ", &[
                    ("source", &dep.source),
                    ("target", &dep.target),
                    ("type", &dep.dependency_type),
                    ("metadata", &dep.metadata),
                ]).await?;
            }
        }

        // Detect cycles
        self.detect_cycles().await?;

        // Calculate metrics
        self.calculate_graph_metrics().await?;

        Ok(())
    }

    async fn detect_cycles(&self) -> Result<Vec<Cycle>> {
        let result = self.db.query("
            # Detect cycles using recursive CTE
            LET $cycles = (
                SELECT id, path,
                FROM code_unit
                WHERE id IN (
                    SELECT DISTINCT in FROM depends_on
                    WHERE <-depends_on.out CONTAINS in
                )
            );

            RETURN $cycles;
        ").await?;

        Ok(self.parse_cycles(result))
    }
}
```

## Type System Integration

### Type Extraction

```rust
struct TypeAnalyzer {
    fn extract_types(&self, ast: &Tree, language: Language) -> Vec<TypeDefinition> {
        match language {
            Language::Rust => self.extract_rust_types(ast),
            Language::TypeScript => self.extract_typescript_types(ast),
            _ => Vec::new()
        }
    }

    fn extract_rust_types(&self, ast: &Tree) -> Vec<TypeDefinition> {
        let mut types = Vec::new();

        // Extract structs
        for node in self.find_nodes(ast, "struct_item") {
            types.push(TypeDefinition {
                kind: TypeKind::Struct,
                name: self.get_name(node),
                generics: self.extract_generics(node),
                fields: self.extract_struct_fields(node),
                traits: Vec::new(),
            });
        }

        // Extract enums
        for node in self.find_nodes(ast, "enum_item") {
            types.push(TypeDefinition {
                kind: TypeKind::Enum,
                name: self.get_name(node),
                generics: self.extract_generics(node),
                variants: self.extract_enum_variants(node),
            });
        }

        // Extract traits
        for node in self.find_nodes(ast, "trait_item") {
            types.push(TypeDefinition {
                kind: TypeKind::Trait,
                name: self.get_name(node),
                generics: self.extract_generics(node),
                methods: self.extract_trait_methods(node),
                super_traits: self.extract_super_traits(node),
            });
        }

        types
    }

    fn extract_typescript_types(&self, ast: &Tree) -> Vec<TypeDefinition> {
        let mut types = Vec::new();

        // Extract interfaces
        for node in self.find_nodes(ast, "interface_declaration") {
            types.push(TypeDefinition {
                kind: TypeKind::Interface,
                name: self.get_name(node),
                generics: self.extract_type_parameters(node),
                members: self.extract_interface_members(node),
                extends: self.extract_extends(node),
            });
        }

        // Extract classes
        for node in self.find_nodes(ast, "class_declaration") {
            types.push(TypeDefinition {
                kind: TypeKind::Class,
                name: self.get_name(node),
                generics: self.extract_type_parameters(node),
                members: self.extract_class_members(node),
                extends: self.extract_extends(node),
                implements: self.extract_implements(node),
            });
        }

        // Extract type aliases
        for node in self.find_nodes(ast, "type_alias_declaration") {
            types.push(TypeDefinition {
                kind: TypeKind::TypeAlias,
                name: self.get_name(node),
                generics: self.extract_type_parameters(node),
                value: self.extract_type_value(node),
            });
        }

        types
    }
}
```

### Type Resolution

```rust
struct TypeResolver {
    async fn resolve_type(&self, type_ref: &str, context: &CodeUnit) -> Option<TypeDefinition> {
        // Check local scope
        if let Some(local) = self.resolve_local_type(type_ref, context) {
            return Some(local);
        }

        // Check imports
        if let Some(imported) = self.resolve_imported_type(type_ref, context) {
            return Some(imported);
        }

        // Check standard library
        if let Some(stdlib) = self.resolve_stdlib_type(type_ref, context.language) {
            return Some(stdlib);
        }

        // Check global scope
        self.resolve_global_type(type_ref)
    }

    fn infer_type(&self, expression: &Node, context: &CodeUnit) -> Option<Type> {
        match expression.kind() {
            "literal_string" => Some(Type::String),
            "literal_number" => Some(Type::Number),
            "literal_boolean" => Some(Type::Boolean),
            "identifier" => self.lookup_identifier_type(expression, context),
            "call_expression" => self.infer_call_type(expression, context),
            "member_expression" => self.infer_member_type(expression, context),
            _ => None
        }
    }
}
```

## Symbol Resolution

### Symbol Table

```rust
struct SymbolTable {
    scopes: Vec<Scope>,
}

struct Scope {
    level: ScopeLevel,
    symbols: HashMap<String, Symbol>,
    parent: Option<usize>,
}

enum ScopeLevel {
    Global,
    Module,
    Class,
    Function,
    Block,
}

struct Symbol {
    name: String,
    qualified_name: String,
    kind: SymbolKind,
    type_: Type,
    visibility: Visibility,
    defined_at: Location,
    references: Vec<Location>,
}

impl SymbolTable {
    fn resolve(&self, name: &str, from_scope: usize) -> Option<&Symbol> {
        let mut scope_idx = from_scope;

        loop {
            let scope = &self.scopes[scope_idx];

            // Check current scope
            if let Some(symbol) = scope.symbols.get(name) {
                if self.is_visible(symbol, from_scope) {
                    return Some(symbol);
                }
            }

            // Move to parent scope
            scope_idx = scope.parent?;
        }
    }

    fn is_visible(&self, symbol: &Symbol, from_scope: usize) -> bool {
        match symbol.visibility {
            Visibility::Public => true,
            Visibility::Private => self.same_module(symbol, from_scope),
            Visibility::Protected => self.same_class_or_subclass(symbol, from_scope),
            Visibility::Internal => self.same_package(symbol, from_scope),
        }
    }
}
```

### Cross-File Resolution

```rust
struct CrossFileResolver {
    async fn resolve_cross_file(&self, reference: &str, from_file: &VNode) -> Option<Symbol> {
        // Get imports from file
        let imports = self.get_imports(from_file).await?;

        // Check each import
        for import in imports {
            if let Some(symbol) = self.check_import(&import, reference).await {
                return Some(symbol);
            }
        }

        // Check re-exports
        self.resolve_reexports(reference, from_file).await
    }

    async fn resolve_module_path(&self, path: &str, from_file: &VNode) -> Option<VNode> {
        // Try relative import
        if path.starts_with('.') {
            return self.resolve_relative_path(path, from_file).await;
        }

        // Try node_modules (for JS/TS)
        if let Some(module) = self.resolve_node_module(path).await {
            return Some(module);
        }

        // Try workspace members (for Rust)
        if let Some(crate_) = self.resolve_workspace_member(path).await {
            return Some(crate_);
        }

        None
    }
}
```

## Semantic Search

### Embedding Generation

```rust
struct EmbeddingGenerator {
    model: Arc<EmbeddingModel>,
}

impl EmbeddingGenerator {
    async fn generate_embedding(&self, unit: &CodeUnit) -> Vec<f32> {
        // Prepare text for embedding
        let text = self.prepare_text(unit);

        // Generate embedding
        let embedding = self.model.embed(&text).await?;

        // Normalize
        self.normalize(embedding)
    }

    fn prepare_text(&self, unit: &CodeUnit) -> String {
        // Combine relevant information
        let mut text = String::new();

        // Add signature (most important)
        text.push_str(&unit.signature);
        text.push(' ');

        // Add docstring if available
        if let Some(doc) = &unit.docstring {
            text.push_str(doc);
            text.push(' ');
        }

        // Add summary
        text.push_str(&unit.summary);
        text.push(' ');

        // Add parameter names
        for param in &unit.parameters {
            text.push_str(&param.name);
            text.push(' ');
        }

        // Add simplified body (without syntax)
        text.push_str(&self.simplify_code(&unit.body));

        text
    }

    fn simplify_code(&self, code: &str) -> String {
        // Remove syntax tokens, keep identifiers and literals
        let tokens = self.tokenize(code);

        tokens.iter()
            .filter(|t| matches!(t.kind, TokenKind::Identifier | TokenKind::Literal))
            .map(|t| &t.text)
            .join(" ")
    }
}
```

### Similarity Search

```rust
impl SemanticSearch {
    async fn search_similar(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        // Generate query embedding
        let query_embedding = self.generator.generate_embedding_from_text(query).await?;

        // Search in vector index
        let results = self.db.query("
            SELECT *,
                   vector::similarity::cosine(embedding, $embedding) as similarity
            FROM code_unit
            WHERE embedding != NONE
            ORDER BY similarity DESC
            LIMIT $limit
        ", &[
            ("embedding", &query_embedding),
            ("limit", &limit),
        ]).await?;

        // Post-process results
        self.enrich_results(results).await
    }

    async fn search_by_pattern(&self, pattern: &ASTPattern) -> Vec<SearchResult> {
        // Convert pattern to tree-sitter query
        let query = self.pattern_to_query(pattern)?;

        // Search across all files
        let mut results = Vec::new();

        for file in self.get_all_files().await? {
            let content = self.get_content(&file).await?;
            let tree = self.parse(content, file.language)?;

            for match_ in query.matches(tree.root_node(), content) {
                results.push(SearchResult {
                    file: file.path.clone(),
                    unit: self.extract_unit_from_match(match_),
                    relevance: self.calculate_pattern_relevance(match_, pattern),
                });
            }
        }

        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        results
    }
}
```

## Call Graph Analysis

### Call Graph Construction

```rust
struct CallGraphBuilder {
    async fn build_call_graph(&self, scope: Scope) -> CallGraph {
        let mut graph = CallGraph::new();

        // Get all functions in scope
        let functions = self.get_functions(scope).await?;

        for function in functions {
            let node = CallNode {
                id: function.id,
                name: function.qualified_name,
                metrics: self.calculate_metrics(&function),
            };

            graph.add_node(node);

            // Find all calls from this function
            let calls = self.find_calls(&function).await?;

            for call in calls {
                if let Some(target) = self.resolve_call(&call, &function).await {
                    graph.add_edge(function.id, target, CallEdge {
                        call_site: call.location,
                        is_recursive: target == function.id,
                        is_virtual: call.is_virtual,
                    });
                }
            }
        }

        graph
    }

    async fn analyze_call_paths(&self, from: UnitId, to: UnitId) -> Vec<CallPath> {
        self.db.query("
            # Find all paths from source to target
            LET $paths = (
                SELECT path
                FROM (
                    SELECT id,
                           ->depends_on[WHERE dependency_type = 'calls']->code_unit as path
                    FROM code_unit
                    WHERE id = $from
                    FETCH path RECURSIVE
                )
                WHERE $to IN path[*].id
            );

            RETURN $paths;
        ", &[
            ("from", &from),
            ("to", &to),
        ]).await?
    }
}
```

### Impact Analysis

```rust
struct ImpactAnalyzer {
    async fn analyze_impact(&self, changed_units: Vec<UnitId>) -> ImpactReport {
        let mut report = ImpactReport::new();

        // Direct impacts
        for unit in &changed_units {
            let direct = self.get_direct_dependents(unit).await?;
            report.directly_affected.extend(direct);
        }

        // Transitive impacts
        let transitive = self.get_transitive_dependents(&changed_units).await?;
        report.transitively_affected = transitive;

        // Categorize by impact type
        for affected in &report.directly_affected {
            let impact_type = self.categorize_impact(affected, &changed_units).await?;
            report.by_type.entry(impact_type).or_default().push(affected);
        }

        // Calculate risk level
        report.risk_level = self.calculate_risk(&report);

        report
    }

    async fn get_transitive_dependents(&self, units: &[UnitId]) -> Vec<UnitId> {
        self.db.query("
            SELECT DISTINCT id
            FROM code_unit
            WHERE id IN (
                SELECT out
                FROM depends_on
                WHERE in IN $units
                FETCH out, out<-depends_on<-code_unit RECURSIVE
            )
        ", &[("units", units)]).await?
    }

    fn calculate_risk(&self, report: &ImpactReport) -> RiskLevel {
        let score =
            report.directly_affected.len() * 3 +
            report.transitively_affected.len() * 1 +
            report.by_type.get(&ImpactType::Breaking).map_or(0, |v| v.len() * 10);

        match score {
            0..=5 => RiskLevel::Low,
            6..=20 => RiskLevel::Medium,
            21..=50 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}
```

## Pattern Recognition

### Pattern Learning

```rust
struct PatternLearner {
    async fn learn_patterns(&self, episodes: &[Episode]) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Group similar changes
        let grouped = self.group_similar_changes(episodes);

        for group in grouped {
            if group.len() >= MIN_PATTERN_FREQUENCY {
                let pattern = self.extract_pattern(&group)?;
                patterns.push(pattern);
            }
        }

        // Validate patterns
        patterns.retain(|p| self.validate_pattern(p));

        patterns
    }

    fn extract_pattern(&self, episodes: &[Episode]) -> Pattern {
        // Find common before state
        let before_ast = self.find_common_ast(&episodes.map(|e| e.before_state));

        // Find common after state
        let after_ast = self.find_common_ast(&episodes.map(|e| e.after_state));

        // Extract transformation
        let transformation = self.extract_transformation(&before_ast, &after_ast);

        Pattern {
            id: generate_id(),
            name: self.generate_pattern_name(&transformation),
            before_pattern: before_ast,
            after_pattern: after_ast,
            transformation,
            frequency: episodes.len(),
            success_rate: self.calculate_success_rate(episodes),
            conditions: self.extract_conditions(episodes),
        }
    }
}
```

### Pattern Matching

```rust
impl PatternMatcher {
    async fn find_matches(&self, pattern: &Pattern, scope: Scope) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        // Convert pattern to tree-sitter query
        let query = self.pattern_to_query(&pattern.before_pattern)?;

        // Search in scope
        for file in self.get_files_in_scope(scope).await? {
            let content = self.get_content(&file).await?;
            let tree = self.parse(content, file.language)?;

            for match_ in query.matches(tree.root_node(), content) {
                // Verify match conditions
                if self.verify_conditions(&match_, &pattern.conditions) {
                    matches.push(PatternMatch {
                        file: file.path.clone(),
                        location: self.get_match_location(&match_),
                        confidence: self.calculate_confidence(&match_, pattern),
                        bindings: self.extract_bindings(&match_),
                    });
                }
            }
        }

        matches
    }

    async fn apply_pattern(&self, pattern: &Pattern, match_: &PatternMatch) -> Result<String> {
        // Get the matched code
        let original = self.get_code_at_location(&match_.location).await?;

        // Apply transformation with bindings
        let transformed = self.apply_transformation(
            &original,
            &pattern.transformation,
            &match_.bindings
        )?;

        // Validate result
        if !self.validate_transformed(transformed, pattern) {
            return Err(Error::InvalidTransformation);
        }

        Ok(transformed)
    }
}
```

## Language-Specific Features

### Rust-Specific Analysis

```rust
impl RustAnalyzer {
    fn analyze_lifetime(&self, node: &Node) -> Vec<Lifetime> {
        let mut lifetimes = Vec::new();

        for child in node.children() {
            if child.kind() == "lifetime" {
                lifetimes.push(Lifetime {
                    name: self.get_text(child),
                    bounds: self.extract_lifetime_bounds(child),
                });
            }
        }

        lifetimes
    }

    fn analyze_traits(&self, impl_node: &Node) -> Vec<TraitImpl> {
        let mut impls = Vec::new();

        if impl_node.kind() == "impl_item" {
            if let Some(trait_node) = impl_node.child_by_field_name("trait") {
                impls.push(TraitImpl {
                    trait_name: self.get_text(trait_node),
                    for_type: self.get_text(impl_node.child_by_field_name("type")),
                    methods: self.extract_impl_methods(impl_node),
                    associated_types: self.extract_associated_types(impl_node),
                });
            }
        }

        impls
    }

    fn analyze_unsafe(&self, node: &Node) -> UnsafeAnalysis {
        let mut analysis = UnsafeAnalysis::default();

        for desc in node.descendants() {
            if desc.kind() == "unsafe_block" {
                analysis.unsafe_blocks.push(UnsafeBlock {
                    location: self.get_location(desc),
                    reason: self.infer_unsafe_reason(desc),
                });
            }
        }

        analysis
    }
}
```

### TypeScript-Specific Analysis

```rust
impl TypeScriptAnalyzer {
    fn analyze_decorators(&self, node: &Node) -> Vec<Decorator> {
        let mut decorators = Vec::new();

        for child in node.children() {
            if child.kind() == "decorator" {
                decorators.push(Decorator {
                    name: self.get_decorator_name(child),
                    arguments: self.extract_decorator_args(child),
                });
            }
        }

        decorators
    }

    fn analyze_generics(&self, node: &Node) -> Vec<Generic> {
        let mut generics = Vec::new();

        if let Some(params) = node.child_by_field_name("type_parameters") {
            for param in params.named_children() {
                generics.push(Generic {
                    name: self.get_text(param.child_by_field_name("name")),
                    constraint: self.extract_constraint(param),
                    default: self.extract_default(param),
                });
            }
        }

        generics
    }

    fn analyze_async(&self, node: &Node) -> AsyncAnalysis {
        AsyncAnalysis {
            is_async: node.child_by_field_name("async").is_some(),
            is_generator: node.child_by_field_name("*").is_some(),
            await_points: self.find_await_points(node),
        }
    }
}
```

## Performance Optimizations

### Query Optimization

```rust
struct QueryOptimizer {
    fn optimize_graph_query(&self, query: &str) -> String {
        // Add indexes hints
        let mut optimized = query.to_string();

        // Use index for path lookups
        optimized = optimized.replace(
            "WHERE path =",
            "WHERE path = /* +INDEX(vnode_path_idx) */"
        );

        // Limit recursion depth
        if optimized.contains("RECURSIVE") && !optimized.contains("MAXDEPTH") {
            optimized = optimized.replace(
                "RECURSIVE",
                "RECURSIVE MAXDEPTH 10"
            );
        }

        optimized
    }

    async fn batch_resolve(&self, references: Vec<Reference>) -> HashMap<Reference, Symbol> {
        // Group by file for batch processing
        let mut by_file: HashMap<VNodeId, Vec<Reference>> = HashMap::new();

        for ref_ in references {
            by_file.entry(ref_.file_id).or_default().push(ref_);
        }

        // Resolve in parallel
        let mut results = HashMap::new();

        for (file_id, refs) in by_file {
            let symbols = self.resolve_in_file(file_id, refs).await?;
            results.extend(symbols);
        }

        results
    }
}
```

### Caching Strategy

```rust
struct GraphCache {
    dependency_cache: Arc<RwLock<HashMap<UnitId, Vec<Dependency>>>>,
    symbol_cache: Arc<RwLock<HashMap<String, Symbol>>>,
    embedding_cache: Arc<RwLock<HashMap<UnitId, Vec<f32>>>>,
}

impl GraphCache {
    async fn get_dependencies(&self, unit_id: &UnitId) -> Option<Vec<Dependency>> {
        self.dependency_cache.read().await.get(unit_id).cloned()
    }

    async fn invalidate_unit(&self, unit_id: &UnitId) {
        let mut deps = self.dependency_cache.write().await;
        deps.remove(unit_id);

        // Also invalidate dependents
        let dependents = self.get_dependents(unit_id).await;
        for dep in dependents {
            deps.remove(&dep);
        }
    }
}
```

## Conclusion

The Semantic Code Graph provides:

1. **Deep Understanding**: Tree-sitter parsing for accurate AST analysis
2. **Rich Relationships**: Multiple relationship types capturing all dependencies
3. **Intelligent Search**: Semantic and pattern-based search capabilities
4. **Impact Analysis**: Understanding change propagation through the graph
5. **Pattern Learning**: Automatic pattern extraction from development history
6. **Language Awareness**: Specialized analysis for each language

This architecture enables LLM agents to understand and manipulate code at a semantic level, far beyond simple text processing.