use crate::types::{Location, SymbolId};
use anyhow::{anyhow, Result};
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

/// Match result from pattern matching
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub symbol_id: Option<SymbolId>,
    pub location: Location,
    pub matched_text: String,
    pub captures: HashMap<String, String>,
    pub score: f32,
}

/// Compiled pattern ready for matching
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    pub original_pattern: String,
    pub tree_sitter_query: String,
    pub language: String,
    pub metavariables: Vec<String>,
}

/// Pattern matcher trait for different languages
pub trait PatternMatcher: Send + Sync {
    /// Compile a structural pattern into a tree-sitter query
    fn compile_pattern(&self, pattern: &str) -> Result<CompiledPattern>;

    /// Match pattern in a file
    fn match_in_file(
        &self,
        pattern: &CompiledPattern,
        content: &str,
        file_path: &Path,
        tree: &Tree,
    ) -> Result<Vec<PatternMatch>>;

    /// Get the language this matcher handles
    fn language(&self) -> &'static str;
}

/// Rust pattern matcher using tree-sitter
pub struct RustPatternMatcher {
    language: Language,
}

impl RustPatternMatcher {
    pub fn new() -> Result<Self> {
        Ok(Self {
            language: tree_sitter_rust::LANGUAGE.into(),
        })
    }

    /// Convert pattern syntax to tree-sitter query
    fn pattern_to_query(&self, pattern: &str) -> Result<(String, Vec<String>)> {
        let query;
        let mut metavars = Vec::new();

        // Parse pattern and convert to tree-sitter query syntax
        // Pattern: "try { ... } catch ($e) { ... }"
        // Tree-sitter query for Rust doesn't have try-catch, but we support match, if, etc.

        // Check for common Rust patterns
        if pattern.contains("fn $name(") {
            // Function pattern: "fn $name($params) { $body }"
            query = r#"
                (function_item
                    name: (identifier) @name
                    parameters: (parameters) @params
                    body: (_) @body) @function
            "#
            .to_string();
            metavars.extend(vec![
                "name".to_string(),
                "params".to_string(),
                "body".to_string(),
            ]);
        } else if pattern.contains("impl $trait for $type") {
            // Trait implementation pattern
            query = r#"
                (impl_item
                    trait: (type_identifier) @trait
                    type: (type_identifier) @type) @impl
            "#
            .to_string();
            metavars.extend(vec!["trait".to_string(), "type".to_string()]);
        } else if pattern.contains("match $expr") {
            // Match expression pattern
            query = r#"
                (match_expression
                    value: (_) @expr) @match
            "#
            .to_string();
            metavars.push("expr".to_string());
        } else if pattern.contains("if $cond") {
            // If expression pattern
            query = r#"
                (if_expression
                    condition: (_) @cond) @if
            "#
            .to_string();
            metavars.push("cond".to_string());
        } else if pattern.contains("struct $name") {
            // Struct pattern
            query = r#"
                (struct_item
                    name: (type_identifier) @name) @struct
            "#
            .to_string();
            metavars.push("name".to_string());
        } else {
            // Generic pattern - try to extract based on common structures
            query = self.generic_rust_pattern(pattern)?;
        }

        Ok((query, metavars))
    }

    fn generic_rust_pattern(&self, pattern: &str) -> Result<String> {
        // Analyze pattern and create appropriate query
        if pattern.contains("...") {
            // Wildcard pattern - match any node
            Ok(r#"(_ ) @match"#.to_string())
        } else {
            Err(anyhow!(
                "Unsupported pattern: {}. Use specific patterns like 'fn $name', 'impl $trait for $type', etc.",
                pattern
            ))
        }
    }
}

impl PatternMatcher for RustPatternMatcher {
    fn compile_pattern(&self, pattern: &str) -> Result<CompiledPattern> {
        let (query, metavars) = self.pattern_to_query(pattern)?;

        // Validate query by trying to compile it
        Query::new(&self.language, &query)
            .map_err(|e| anyhow!("Invalid tree-sitter query: {}", e))?;

        Ok(CompiledPattern {
            original_pattern: pattern.to_string(),
            tree_sitter_query: query,
            language: "rust".to_string(),
            metavariables: metavars,
        })
    }

    fn match_in_file(
        &self,
        pattern: &CompiledPattern,
        content: &str,
        file_path: &Path,
        tree: &Tree,
    ) -> Result<Vec<PatternMatch>> {
        let query = Query::new(&self.language, &pattern.tree_sitter_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut matches = Vec::new();

        let mut query_matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = query_matches.next() {
            let mut captures = HashMap::new();
            let mut primary_node = None;

            for capture in m.captures {
                let node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];

                if primary_node.is_none() {
                    primary_node = Some(node);
                }

                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    captures.insert(capture_name.to_string(), text.to_string());
                }
            }

            if let Some(node) = primary_node {
                let start = node.start_position();
                let end = node.end_position();

                let location = Location::new(
                    file_path.to_string_lossy().to_string(),
                    start.row + 1,
                    end.row + 1,
                    start.column,
                    end.column,
                );

                let matched_text = node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                matches.push(PatternMatch {
                    symbol_id: None,
                    location,
                    matched_text,
                    captures,
                    score: 1.0, // Exact match
                });
            }
        }

        Ok(matches)
    }

    fn language(&self) -> &'static str {
        "rust"
    }
}

/// TypeScript/TSX pattern matcher
pub struct TypeScriptPatternMatcher {
    language: Language,
    is_tsx: bool,
}

impl TypeScriptPatternMatcher {
    pub fn new(is_tsx: bool) -> Result<Self> {
        let language = if is_tsx {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
        };

        Ok(Self { language, is_tsx })
    }

    fn pattern_to_query(&self, pattern: &str) -> Result<(String, Vec<String>)> {
        let query;
        let mut metavars = Vec::new();

        if pattern.contains("try {") && pattern.contains("catch") {
            // Try-catch pattern: "try { ... } catch ($e) { ... }"
            query = r#"
                (try_statement
                    body: (statement_block) @try_body
                    handler: (catch_clause
                        parameter: (_)? @catch_param
                        body: (statement_block) @catch_body)) @try_catch
            "#
            .to_string();
            metavars.extend(vec![
                "try_body".to_string(),
                "catch_param".to_string(),
                "catch_body".to_string(),
            ]);
        } else if pattern.contains("function $name(") {
            // Function pattern
            query = r#"
                (function_declaration
                    name: (identifier) @name
                    parameters: (formal_parameters) @params
                    body: (statement_block) @body) @function
            "#
            .to_string();
            metavars.extend(vec![
                "name".to_string(),
                "params".to_string(),
                "body".to_string(),
            ]);
        } else if pattern.contains("class $name") {
            // Class pattern
            query = r#"
                (class_declaration
                    name: (type_identifier) @name
                    body: (class_body) @body) @class
            "#
            .to_string();
            metavars.extend(vec!["name".to_string(), "body".to_string()]);
        } else if pattern.contains("interface $name") {
            // Interface pattern
            query = r#"
                (interface_declaration
                    name: (type_identifier) @name
                    body: (object_type) @body) @interface
            "#
            .to_string();
            metavars.extend(vec!["name".to_string(), "body".to_string()]);
        } else if pattern.contains("async ") {
            // Async function/method pattern
            query = r#"
                [
                    (function_declaration
                        (async) @async
                        name: (identifier) @name) @function
                    (method_definition
                        (async) @async
                        name: (property_identifier) @name) @method
                ]
            "#
            .to_string();
            metavars.extend(vec!["name".to_string()]);
        } else {
            query = self.generic_typescript_pattern(pattern)?;
        }

        Ok((query, metavars))
    }

    fn generic_typescript_pattern(&self, pattern: &str) -> Result<String> {
        if pattern.contains("...") {
            Ok(r#"(_ ) @match"#.to_string())
        } else {
            Err(anyhow!(
                "Unsupported TypeScript pattern: {}. Use specific patterns like 'try {{ ... }} catch', 'function $name', etc.",
                pattern
            ))
        }
    }
}

impl PatternMatcher for TypeScriptPatternMatcher {
    fn compile_pattern(&self, pattern: &str) -> Result<CompiledPattern> {
        let (query, metavars) = self.pattern_to_query(pattern)?;

        Query::new(&self.language, &query)
            .map_err(|e| anyhow!("Invalid tree-sitter query: {}", e))?;

        Ok(CompiledPattern {
            original_pattern: pattern.to_string(),
            tree_sitter_query: query,
            language: if self.is_tsx { "tsx".to_string() } else { "typescript".to_string() },
            metavariables: metavars,
        })
    }

    fn match_in_file(
        &self,
        pattern: &CompiledPattern,
        content: &str,
        file_path: &Path,
        tree: &Tree,
    ) -> Result<Vec<PatternMatch>> {
        let query = Query::new(&self.language, &pattern.tree_sitter_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut matches = Vec::new();

        let mut query_matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = query_matches.next() {
            let mut captures = HashMap::new();
            let mut primary_node = None;

            for capture in m.captures {
                let node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];

                if primary_node.is_none() {
                    primary_node = Some(node);
                }

                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    captures.insert(capture_name.to_string(), text.to_string());
                }
            }

            if let Some(node) = primary_node {
                let start = node.start_position();
                let end = node.end_position();

                let location = Location::new(
                    file_path.to_string_lossy().to_string(),
                    start.row + 1,
                    end.row + 1,
                    start.column,
                    end.column,
                );

                let matched_text = node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                matches.push(PatternMatch {
                    symbol_id: None,
                    location,
                    matched_text,
                    captures,
                    score: 1.0,
                });
            }
        }

        Ok(matches)
    }

    fn language(&self) -> &'static str {
        if self.is_tsx {
            "tsx"
        } else {
            "typescript"
        }
    }
}

/// JavaScript pattern matcher
pub struct JavaScriptPatternMatcher {
    language: Language,
}

impl JavaScriptPatternMatcher {
    pub fn new() -> Result<Self> {
        Ok(Self {
            language: tree_sitter_javascript::LANGUAGE.into(),
        })
    }

    fn pattern_to_query(&self, pattern: &str) -> Result<(String, Vec<String>)> {
        // Similar to TypeScript but without type annotations
        let query;
        let mut metavars = Vec::new();

        if pattern.contains("try {") && pattern.contains("catch") {
            query = r#"
                (try_statement
                    body: (statement_block) @try_body
                    handler: (catch_clause
                        parameter: (_)? @catch_param
                        body: (statement_block) @catch_body)) @try_catch
            "#
            .to_string();
            metavars.extend(vec![
                "try_body".to_string(),
                "catch_param".to_string(),
                "catch_body".to_string(),
            ]);
        } else if pattern.contains("function $name(") {
            query = r#"
                (function_declaration
                    name: (identifier) @name
                    parameters: (formal_parameters) @params
                    body: (statement_block) @body) @function
            "#
            .to_string();
            metavars.extend(vec![
                "name".to_string(),
                "params".to_string(),
                "body".to_string(),
            ]);
        } else if pattern.contains("class $name") {
            query = r#"
                (class_declaration
                    name: (identifier) @name
                    body: (class_body) @body) @class
            "#
            .to_string();
            metavars.extend(vec!["name".to_string(), "body".to_string()]);
        } else {
            query = r#"(_ ) @match"#.to_string();
        }

        Ok((query, metavars))
    }
}

impl PatternMatcher for JavaScriptPatternMatcher {
    fn compile_pattern(&self, pattern: &str) -> Result<CompiledPattern> {
        let (query, metavars) = self.pattern_to_query(pattern)?;

        Query::new(&self.language, &query)
            .map_err(|e| anyhow!("Invalid tree-sitter query: {}", e))?;

        Ok(CompiledPattern {
            original_pattern: pattern.to_string(),
            tree_sitter_query: query,
            language: "javascript".to_string(),
            metavariables: metavars,
        })
    }

    fn match_in_file(
        &self,
        pattern: &CompiledPattern,
        content: &str,
        file_path: &Path,
        tree: &Tree,
    ) -> Result<Vec<PatternMatch>> {
        let query = Query::new(&self.language, &pattern.tree_sitter_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut matches = Vec::new();

        let mut query_matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = query_matches.next() {
            let mut captures = HashMap::new();
            let mut primary_node = None;

            for capture in m.captures {
                let node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];

                if primary_node.is_none() {
                    primary_node = Some(node);
                }

                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    captures.insert(capture_name.to_string(), text.to_string());
                }
            }

            if let Some(node) = primary_node {
                let start = node.start_position();
                let end = node.end_position();

                let location = Location::new(
                    file_path.to_string_lossy().to_string(),
                    start.row + 1,
                    end.row + 1,
                    start.column,
                    end.column,
                );

                let matched_text = node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                matches.push(PatternMatch {
                    symbol_id: None,
                    location,
                    matched_text,
                    captures,
                    score: 1.0,
                });
            }
        }

        Ok(matches)
    }

    fn language(&self) -> &'static str {
        "javascript"
    }
}

/// Python pattern matcher
pub struct PythonPatternMatcher {
    language: Language,
}

impl PythonPatternMatcher {
    pub fn new() -> Result<Self> {
        Ok(Self {
            language: tree_sitter_python::LANGUAGE.into(),
        })
    }

    fn pattern_to_query(&self, pattern: &str) -> Result<(String, Vec<String>)> {
        let query;
        let mut metavars = Vec::new();

        if pattern.contains("try:") && pattern.contains("except") {
            query = r#"
                (try_statement
                    body: (block) @try_body
                    (except_clause
                        (_)? @exception_type
                        (as (_) @exception_var)?
                        consequence: (block) @except_body)) @try_except
            "#
            .to_string();
            metavars.extend(vec![
                "try_body".to_string(),
                "exception_type".to_string(),
                "except_body".to_string(),
            ]);
        } else if pattern.contains("def $name(") {
            query = r#"
                (function_definition
                    name: (identifier) @name
                    parameters: (parameters) @params
                    body: (block) @body) @function
            "#
            .to_string();
            metavars.extend(vec![
                "name".to_string(),
                "params".to_string(),
                "body".to_string(),
            ]);
        } else if pattern.contains("class $name") {
            query = r#"
                (class_definition
                    name: (identifier) @name
                    body: (block) @body) @class
            "#
            .to_string();
            metavars.extend(vec!["name".to_string(), "body".to_string()]);
        } else if pattern.contains("async def") {
            query = r#"
                (function_definition
                    (async) @async
                    name: (identifier) @name) @function
            "#
            .to_string();
            metavars.push("name".to_string());
        } else {
            query = r#"(_ ) @match"#.to_string();
        }

        Ok((query, metavars))
    }
}

impl PatternMatcher for PythonPatternMatcher {
    fn compile_pattern(&self, pattern: &str) -> Result<CompiledPattern> {
        let (query, metavars) = self.pattern_to_query(pattern)?;

        Query::new(&self.language, &query)
            .map_err(|e| anyhow!("Invalid tree-sitter query: {}", e))?;

        Ok(CompiledPattern {
            original_pattern: pattern.to_string(),
            tree_sitter_query: query,
            language: "python".to_string(),
            metavariables: metavars,
        })
    }

    fn match_in_file(
        &self,
        pattern: &CompiledPattern,
        content: &str,
        file_path: &Path,
        tree: &Tree,
    ) -> Result<Vec<PatternMatch>> {
        let query = Query::new(&self.language, &pattern.tree_sitter_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut matches = Vec::new();

        let mut query_matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = query_matches.next() {
            let mut captures = HashMap::new();
            let mut primary_node = None;

            for capture in m.captures {
                let node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];

                if primary_node.is_none() {
                    primary_node = Some(node);
                }

                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    captures.insert(capture_name.to_string(), text.to_string());
                }
            }

            if let Some(node) = primary_node {
                let start = node.start_position();
                let end = node.end_position();

                let location = Location::new(
                    file_path.to_string_lossy().to_string(),
                    start.row + 1,
                    end.row + 1,
                    start.column,
                    end.column,
                );

                let matched_text = node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                matches.push(PatternMatch {
                    symbol_id: None,
                    location,
                    matched_text,
                    captures,
                    score: 1.0,
                });
            }
        }

        Ok(matches)
    }

    fn language(&self) -> &'static str {
        "python"
    }
}

/// Go pattern matcher
pub struct GoPatternMatcher {
    language: Language,
}

impl GoPatternMatcher {
    pub fn new() -> Result<Self> {
        Ok(Self {
            language: tree_sitter_go::LANGUAGE.into(),
        })
    }

    fn pattern_to_query(&self, pattern: &str) -> Result<(String, Vec<String>)> {
        let query;
        let mut metavars = Vec::new();

        if pattern.contains("func $name(") {
            query = r#"
                (function_declaration
                    name: (identifier) @name
                    parameters: (parameter_list) @params
                    result: (_)? @result
                    body: (block) @body) @function
            "#
            .to_string();
            metavars.extend(vec![
                "name".to_string(),
                "params".to_string(),
                "body".to_string(),
            ]);
        } else if pattern.contains("type $name struct") {
            query = r#"
                (type_declaration
                    (type_spec
                        name: (type_identifier) @name
                        type: (struct_type) @struct_type)) @type_decl
            "#
            .to_string();
            metavars.push("name".to_string());
        } else if pattern.contains("interface $name") || pattern.contains("type $name interface") {
            query = r#"
                (type_declaration
                    (type_spec
                        name: (type_identifier) @name
                        type: (interface_type) @interface_type)) @type_decl
            "#
            .to_string();
            metavars.push("name".to_string());
        } else if pattern.contains("defer ") {
            query = r#"
                (defer_statement
                    (_) @deferred) @defer
            "#
            .to_string();
            metavars.push("deferred".to_string());
        } else if pattern.contains("go ") {
            query = r#"
                (go_statement
                    (_) @goroutine) @go
            "#
            .to_string();
            metavars.push("goroutine".to_string());
        } else {
            query = r#"(_ ) @match"#.to_string();
        }

        Ok((query, metavars))
    }
}

impl PatternMatcher for GoPatternMatcher {
    fn compile_pattern(&self, pattern: &str) -> Result<CompiledPattern> {
        let (query, metavars) = self.pattern_to_query(pattern)?;

        Query::new(&self.language, &query)
            .map_err(|e| anyhow!("Invalid tree-sitter query: {}", e))?;

        Ok(CompiledPattern {
            original_pattern: pattern.to_string(),
            tree_sitter_query: query,
            language: "go".to_string(),
            metavariables: metavars,
        })
    }

    fn match_in_file(
        &self,
        pattern: &CompiledPattern,
        content: &str,
        file_path: &Path,
        tree: &Tree,
    ) -> Result<Vec<PatternMatch>> {
        let query = Query::new(&self.language, &pattern.tree_sitter_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut matches = Vec::new();

        let mut query_matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = query_matches.next() {
            let mut captures = HashMap::new();
            let mut primary_node = None;

            for capture in m.captures {
                let node = capture.node;
                let capture_name = query.capture_names()[capture.index as usize];

                if primary_node.is_none() {
                    primary_node = Some(node);
                }

                if let Ok(text) = node.utf8_text(content.as_bytes()) {
                    captures.insert(capture_name.to_string(), text.to_string());
                }
            }

            if let Some(node) = primary_node {
                let start = node.start_position();
                let end = node.end_position();

                let location = Location::new(
                    file_path.to_string_lossy().to_string(),
                    start.row + 1,
                    end.row + 1,
                    start.column,
                    end.column,
                );

                let matched_text = node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                matches.push(PatternMatch {
                    symbol_id: None,
                    location,
                    matched_text,
                    captures,
                    score: 1.0,
                });
            }
        }

        Ok(matches)
    }

    fn language(&self) -> &'static str {
        "go"
    }
}

/// Pattern search engine with caching
pub struct PatternSearchEngine {
    matchers: HashMap<&'static str, Arc<dyn PatternMatcher>>,
    pattern_cache: Arc<Mutex<LruCache<String, CompiledPattern>>>,
    parsers: Arc<Mutex<HashMap<&'static str, Parser>>>,
}

impl PatternSearchEngine {
    pub fn new() -> Result<Self> {
        let mut matchers: HashMap<&'static str, Arc<dyn PatternMatcher>> = HashMap::new();

        // Initialize all language matchers
        matchers.insert("rust", Arc::new(RustPatternMatcher::new()?));
        matchers.insert(
            "typescript",
            Arc::new(TypeScriptPatternMatcher::new(false)?),
        );
        matchers.insert("tsx", Arc::new(TypeScriptPatternMatcher::new(true)?));
        matchers.insert("javascript", Arc::new(JavaScriptPatternMatcher::new()?));
        matchers.insert("python", Arc::new(PythonPatternMatcher::new()?));
        matchers.insert("go", Arc::new(GoPatternMatcher::new()?));

        // Create pattern cache (1000 entries)
        let cache_size = NonZeroUsize::new(1000).unwrap();
        let pattern_cache = Arc::new(Mutex::new(LruCache::new(cache_size)));

        // Initialize parsers for each language
        let mut parsers_map = HashMap::new();
        for &lang in matchers.keys() {
            parsers_map.insert(lang, Parser::new());
        }
        let parsers = Arc::new(Mutex::new(parsers_map));

        Ok(Self {
            matchers,
            pattern_cache,
            parsers,
        })
    }

    /// Compile a pattern for a specific language (with caching)
    pub fn compile_pattern(&self, pattern: &str, language: &str) -> Result<CompiledPattern> {
        // Check cache first
        let cache_key = format!("{}:{}", language, pattern);

        {
            let mut cache = self.pattern_cache.lock();
            if let Some(compiled) = cache.get(&cache_key) {
                return Ok(compiled.clone());
            }
        }

        // Compile pattern
        let matcher = self
            .matchers
            .get(language)
            .ok_or_else(|| anyhow!("Unsupported language: {}", language))?;

        let compiled = matcher.compile_pattern(pattern)?;

        // Store in cache
        {
            let mut cache = self.pattern_cache.lock();
            cache.put(cache_key, compiled.clone());
        }

        Ok(compiled)
    }

    /// Search for pattern in a file
    pub fn search_in_file(
        &self,
        pattern: &str,
        language: &str,
        content: &str,
        file_path: &Path,
    ) -> Result<Vec<PatternMatch>> {
        // Compile pattern
        let compiled = self.compile_pattern(pattern, language)?;

        // Get matcher
        let matcher = self
            .matchers
            .get(language)
            .ok_or_else(|| anyhow!("Unsupported language: {}", language))?;

        // Parse the file
        let mut parsers = self.parsers.lock();
        let parser = parsers
            .get_mut(language)
            .ok_or_else(|| anyhow!("No parser for language: {}", language))?;

        // Set language
        let lang_obj = match language {
            "rust" => tree_sitter_rust::LANGUAGE.into(),
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
            "javascript" => tree_sitter_javascript::LANGUAGE.into(),
            "python" => tree_sitter_python::LANGUAGE.into(),
            "go" => tree_sitter_go::LANGUAGE.into(),
            _ => return Err(anyhow!("Unsupported language: {}", language)),
        };

        parser
            .set_language(&lang_obj)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse file"))?;

        // Match pattern
        matcher.match_in_file(&compiled, content, file_path, &tree)
    }

    /// Detect language from file extension
    pub fn detect_language(file_path: &Path) -> Result<&'static str> {
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("No file extension"))?;

        match ext {
            "rs" => Ok("rust"),
            "ts" => Ok("typescript"),
            "tsx" => Ok("tsx"),
            "js" | "jsx" => Ok("javascript"),
            "py" => Ok("python"),
            "go" => Ok("go"),
            _ => Err(anyhow!("Unsupported file extension: {}", ext)),
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.pattern_cache.lock();
        (cache.len(), cache.cap().get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rust_function_pattern() {
        let engine = PatternSearchEngine::new().unwrap();
        let content = r#"
            pub fn test_function(x: i32, y: i32) -> i32 {
                x + y
            }

            pub fn another_function() {
                println!("hello");
            }
        "#;

        let path = PathBuf::from("test.rs");
        let matches = engine
            .search_in_file("fn $name($params)", "rust", content, &path)
            .unwrap();

        assert!(!matches.is_empty());
        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_typescript_try_catch_pattern() {
        let engine = PatternSearchEngine::new().unwrap();
        let content = r#"
            function testFunc() {
                try {
                    riskyOperation();
                } catch (error) {
                    console.error(error);
                }
            }

            async function anotherFunc() {
                try {
                    await asyncOp();
                } catch (e) {
                    handleError(e);
                }
            }
        "#;

        let path = PathBuf::from("test.ts");
        let matches = engine
            .search_in_file("try { ... } catch ($e) { ... }", "typescript", content, &path)
            .unwrap();

        assert_eq!(matches.len(), 2);
        assert!(matches[0].captures.contains_key("catch_param"));
    }

    #[test]
    fn test_pattern_cache() {
        let engine = PatternSearchEngine::new().unwrap();

        // Compile same pattern twice
        let pattern = "fn $name($params)";
        engine.compile_pattern(pattern, "rust").unwrap();
        engine.compile_pattern(pattern, "rust").unwrap();

        let (cached, _) = engine.cache_stats();
        assert_eq!(cached, 1); // Should be cached
    }

    #[test]
    fn test_python_pattern() {
        let engine = PatternSearchEngine::new().unwrap();
        let content = r#"
def test_function(x, y):
    return x + y

class TestClass:
    def method(self):
        pass
        "#;

        let path = PathBuf::from("test.py");
        let matches = engine
            .search_in_file("def $name($params)", "python", content, &path)
            .unwrap();

        assert!(!matches.is_empty());
    }

    #[test]
    fn test_go_pattern() {
        let engine = PatternSearchEngine::new().unwrap();
        let content = r#"
package main

func testFunction(x int, y int) int {
    return x + y
}

func anotherFunction() {
    defer cleanup()
    doSomething()
}
        "#;

        let path = PathBuf::from("test.go");

        // Test function pattern
        let func_matches = engine
            .search_in_file("func $name($params)", "go", content, &path)
            .unwrap();
        assert!(!func_matches.is_empty());

        // Test defer pattern
        let defer_matches = engine
            .search_in_file("defer ", "go", content, &path)
            .unwrap();
        assert_eq!(defer_matches.len(), 1);
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(
            PatternSearchEngine::detect_language(&PathBuf::from("test.rs")).unwrap(),
            "rust"
        );
        assert_eq!(
            PatternSearchEngine::detect_language(&PathBuf::from("test.ts")).unwrap(),
            "typescript"
        );
        assert_eq!(
            PatternSearchEngine::detect_language(&PathBuf::from("test.py")).unwrap(),
            "python"
        );
    }

    #[test]
    fn test_javascript_pattern() {
        let engine = PatternSearchEngine::new().unwrap();
        let content = r#"
            function testFunc() {
                try {
                    riskyOp();
                } catch (err) {
                    console.error(err);
                }
            }

            class MyClass {
                constructor() {}
            }
        "#;

        let path = PathBuf::from("test.js");
        let matches = engine
            .search_in_file("try { ... } catch ($e) { ... }", "javascript", content, &path)
            .unwrap();

        assert_eq!(matches.len(), 1);
    }
}
