use super::{Indexer, TreeSitterParser, IgnoreMatcher};
use crate::cache::{MultiLevelCache, MultiLevelCacheConfig};
use crate::config::IndexConfig;
use crate::embeddings::EmbeddingEngine;
use crate::storage::{deserialize, serialize, Storage};
use crate::types::{
    CodeSymbol, DetailLevel, Query, QueryResult, Reference, ReferenceKind,
    SymbolDefinition, SymbolId, TokenCount,
};
use anyhow::{Context, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use std::time::SystemTime;

/// Dependency graph edge
#[derive(Debug, Clone)]
struct Dependency {
    from: SymbolId,
    to: SymbolId,
    kind: ReferenceKind,
}

/// Dependency graph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<SymbolId>,
    pub edges: Vec<(SymbolId, SymbolId, ReferenceKind)>,
}

/// Cache key for search results
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct SearchCacheKey {
    text: String,
    symbol_types: Option<Vec<String>>,
    scope: Option<String>,
    detail_level: String,
    max_results: Option<usize>,
    offset: Option<usize>,
}

impl SearchCacheKey {
    fn from_query(query: &Query) -> Self {
        Self {
            text: query.text.clone(),
            symbol_types: query.symbol_types.as_ref().map(|types| {
                types.iter().map(|t| t.as_str().to_string()).collect()
            }),
            scope: query.scope.clone(),
            detail_level: format!("{:?}", query.detail_level),
            max_results: query.max_results,
            offset: query.offset,
        }
    }
}

pub struct CodeIndexer {
    storage: Arc<dyn Storage>,
    config: IndexConfig,
    parser: TreeSitterParser,
    // In-memory cache for fast lookups
    symbols: DashMap<SymbolId, CodeSymbol>,
    // Symbol name to ID mapping for fast lookup
    name_index: DashMap<String, Vec<SymbolId>>,
    // File to symbols mapping
    file_index: DashMap<PathBuf, Vec<SymbolId>>,
    // Dependency graph
    dependencies: DashMap<SymbolId, Vec<Dependency>>,
    // Source code cache for getting full definitions
    source_cache: DashMap<PathBuf, String>,
    // Embedding engine for semantic search
    embedding_engine: Arc<parking_lot::Mutex<EmbeddingEngine>>,
    // Multi-level cache for search results (L1: 1K, L2: 10K, L3: RocksDB)
    search_cache: Arc<MultiLevelCache<SearchCacheKey, QueryResult>>,
    // File modification time tracking for incremental indexing
    file_mtimes: DashMap<PathBuf, SystemTime>,
    // Ignore matcher for gitignore support (async mutex for async load_gitignore)
    ignore_matcher: Arc<TokioMutex<IgnoreMatcher>>,
}

impl CodeIndexer {
    pub fn new(storage: Arc<dyn Storage>, config: IndexConfig) -> Result<Self> {
        let embedding_engine = Arc::new(parking_lot::Mutex::new(EmbeddingEngine::new()?));

        // Initialize multi-level cache with optimized configuration
        let cache_config = MultiLevelCacheConfig {
            l1_capacity: 1_000,    // Hot: 1K entries, <1ms access
            l2_capacity: 10_000,   // Warm: 10K entries, 1-5ms access
            l3_prefix: "search_cache:".to_string(),
            auto_promote: true,
        };
        let search_cache = Arc::new(MultiLevelCache::new(storage.clone(), cache_config)?);
        let ignore_matcher = Arc::new(TokioMutex::new(IgnoreMatcher::new()));

        Ok(Self {
            storage,
            config,
            parser: TreeSitterParser::new()?,
            symbols: DashMap::new(),
            name_index: DashMap::new(),
            file_index: DashMap::new(),
            dependencies: DashMap::new(),
            source_cache: DashMap::new(),
            embedding_engine,
            search_cache,
            file_mtimes: DashMap::new(),
            ignore_matcher,
        })
    }

    /// Load existing index from storage
    pub async fn load(&mut self) -> Result<()> {
        let keys = self.storage.get_keys_with_prefix(b"symbol:").await?;

        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                let symbol: CodeSymbol = deserialize(&data)?;
                let symbol_id = symbol.id.clone();
                let symbol_name = symbol.name.clone();
                let symbol_file = PathBuf::from(symbol.location.file.clone());

                // Add to symbol cache
                self.symbols.insert(symbol_id.clone(), symbol.clone());

                // Add to name index
                self.name_index
                    .entry(symbol_name)
                    .or_default()
                    .push(symbol_id.clone());

                // Add to file index
                self.file_index
                    .entry(symbol_file)
                    .or_default()
                    .push(symbol_id.clone());

                // Build dependency graph
                for dep in &symbol.dependencies {
                    self.dependencies
                        .entry(symbol_id.clone())
                        .or_default()
                        .push(Dependency {
                            from: symbol_id.clone(),
                            to: dep.clone(),
                            kind: ReferenceKind::TypeReference,
                        });
                }
            }
        }

        tracing::info!("Loaded {} symbols from storage", self.symbols.len());
        Ok(())
    }

    /// Get the total number of indexed symbols
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Get the total number of indexed files
    pub fn file_count(&self) -> usize {
        self.file_index.len()
    }

    /// Get multi-level cache statistics
    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.search_cache.stats()
    }

    /// Get cache sizes (L1, L2)
    pub fn cache_sizes(&self) -> (usize, usize) {
        self.search_cache.sizes()
    }

    /// Check if file has been modified since last index
    fn has_file_changed(&self, path: &Path) -> Result<bool> {
        let metadata = std::fs::metadata(path)?;
        let current_mtime = metadata.modified()?;

        if let Some(cached_mtime) = self.file_mtimes.get(path) {
            Ok(current_mtime > *cached_mtime)
        } else {
            Ok(true) // File not indexed yet
        }
    }

    /// Index a file incrementally - only if it has changed
    pub async fn index_file_incremental(&mut self, path: &Path) -> Result<bool> {
        // Check if file has changed
        match self.has_file_changed(path) {
            Ok(false) => {
                tracing::debug!("Skipping unchanged file: {:?}", path);
                return Ok(false); // File not changed
            }
            Ok(true) => {
                tracing::debug!("Re-indexing changed file: {:?}", path);
            }
            Err(e) => {
                tracing::warn!("Failed to check file modification time for {:?}: {}", path, e);
                // On error, re-index the file to be safe
            }
        }

        // Use incremental parsing
        self.index_file_incremental_parse(path).await?;

        Ok(true) // File was re-indexed
    }

    /// Index a file using incremental parsing (tree-sitter)
    async fn index_file_incremental_parse(&mut self, path: &Path) -> Result<()> {
        use super::tree_sitter_parser::TreeSitterParser;

        // Check if file should be ignored
        if self.should_ignore(path) {
            return Ok(());
        }

        // Read file content
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        // Store modification time for incremental indexing
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(mtime) = metadata.modified() {
                self.file_mtimes.insert(path.to_path_buf(), mtime);
            }
        }

        // Cache the source
        self.source_cache
            .insert(path.to_path_buf(), content.clone());

        // Parse incrementally
        let parse_result = self.parser.parse_file_incremental(path, &content)?;

        tracing::debug!(
            "Incremental parse: {} symbols, {} changed ranges",
            parse_result.symbols.len(),
            parse_result.changed_ranges.len()
        );

        // Get old symbols for this file
        let old_symbol_ids: Vec<SymbolId> = self
            .file_index
            .get(path)
            .map(|ids| ids.clone())
            .unwrap_or_default();

        // Build set of symbols that need updating
        let mut symbols_to_update = Vec::new();
        let mut symbols_to_keep = std::collections::HashSet::new();

        for new_symbol in &parse_result.symbols {
            // Check if this symbol is in a changed range
            if TreeSitterParser::symbol_in_changed_range(new_symbol, &parse_result.changed_ranges) {
                symbols_to_update.push(new_symbol.clone());
            } else {
                // Symbol unchanged - see if we have it cached
                if let Some(old_id) = old_symbol_ids
                    .iter()
                    .find(|id| {
                        if let Some(old_sym) = self.symbols.get(id) {
                            old_sym.name == new_symbol.name
                                && old_sym.location.line_start == new_symbol.location.line_start
                        } else {
                            false
                        }
                    })
                {
                    symbols_to_keep.insert(old_id.clone());
                }
            }
        }

        // Remove symbols that are no longer present or changed
        for old_id in old_symbol_ids {
            if !symbols_to_keep.contains(&old_id) {
                self.symbols.remove(&old_id);

                // Remove from name index
                for mut entry in self.name_index.iter_mut() {
                    entry.value_mut().retain(|id| id != &old_id);
                }

                // Remove from dependencies
                self.dependencies.remove(&old_id);
            }
        }

        // Build dependencies for new symbols
        self.build_local_dependencies(&mut symbols_to_update);

        // Generate embeddings for updated symbols only
        for symbol in &mut symbols_to_update {
            // Create embedding text from symbol name, signature, and doc comment
            let embedding_text = format!(
                "{} {} {}",
                symbol.name,
                symbol.signature,
                symbol.metadata.doc_comment.as_deref().unwrap_or("")
            );

            // Generate embedding
            match self.embedding_engine.lock().generate_embedding(&embedding_text) {
                Ok(embedding) => {
                    symbol.embedding = Some(embedding);
                }
                Err(e) => {
                    tracing::warn!("Failed to generate embedding for symbol {}: {}", symbol.name, e);
                }
            }
        }

        // Store only updated symbols
        for symbol in &symbols_to_update {
            let key = format!("symbol:{}", symbol.id.0);
            let value = serialize(symbol)?;
            self.storage.put(key.as_bytes(), &value).await?;

            // Update caches
            self.symbols.insert(symbol.id.clone(), symbol.clone());

            self.name_index
                .entry(symbol.name.clone())
                .or_default()
                .push(symbol.id.clone());
        }

        // Rebuild file index
        let all_symbol_ids: Vec<SymbolId> = symbols_to_keep
            .into_iter()
            .chain(symbols_to_update.iter().map(|s| s.id.clone()))
            .collect();

        self.file_index.insert(path.to_path_buf(), all_symbol_ids);

        tracing::debug!(
            "Incremental update: {} symbols updated for {:?}",
            symbols_to_update.len(),
            path
        );

        Ok(())
    }

    /// Index a single file
    #[allow(dead_code)]
    async fn index_file(&mut self, path: &Path) -> Result<Vec<CodeSymbol>> {
        // Check if file should be ignored
        if self.should_ignore(path) {
            return Ok(Vec::new());
        }

        // Read file content
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        // Store modification time for incremental indexing
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(mtime) = metadata.modified() {
                self.file_mtimes.insert(path.to_path_buf(), mtime);
            }
        }

        // Cache the source
        self.source_cache
            .insert(path.to_path_buf(), content.clone());

        // Parse and extract symbols
        let mut symbols = self.parser.parse_file(path, &content)?;

        // Build dependencies between symbols in this file
        self.build_local_dependencies(&mut symbols);

        // Generate embeddings for each symbol
        for symbol in &mut symbols {
            // Create embedding text from symbol name, signature, and doc comment
            let embedding_text = format!(
                "{} {} {}",
                symbol.name,
                symbol.signature,
                symbol.metadata.doc_comment.as_deref().unwrap_or("")
            );

            // Generate embedding
            match self.embedding_engine.lock().generate_embedding(&embedding_text) {
                Ok(embedding) => {
                    symbol.embedding = Some(embedding);
                }
                Err(e) => {
                    tracing::warn!("Failed to generate embedding for symbol {}: {}", symbol.name, e);
                }
            }
        }

        // Store symbols
        for symbol in &symbols {
            let key = format!("symbol:{}", symbol.id.0);
            let value = serialize(symbol)?;
            self.storage.put(key.as_bytes(), &value).await?;

            // Update caches
            self.symbols.insert(symbol.id.clone(), symbol.clone());

            self.name_index
                .entry(symbol.name.clone())
                .or_default()
                .push(symbol.id.clone());

            self.file_index
                .entry(path.to_path_buf())
                .or_default()
                .push(symbol.id.clone());
        }

        Ok(symbols)
    }

    /// Build dependencies between symbols in the same file
    fn build_local_dependencies(&mut self, symbols: &mut [CodeSymbol]) {
        // Build reverse lookup from symbol ID to name
        let id_to_name: HashMap<SymbolId, String> = symbols
            .iter()
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect();

        let name_to_id: HashMap<String, SymbolId> = symbols
            .iter()
            .map(|s| (s.name.clone(), s.id.clone()))
            .collect();

        for symbol in symbols.iter_mut() {
            let mut deps = Vec::new();

            // Check references to other symbols
            for reference in &symbol.references.clone() {
                if let Some(name) = id_to_name.get(&reference.symbol_id) {
                    if let Some(target_id) = name_to_id.get(name) {
                        deps.push(target_id.clone());

                        // Also add to dependency graph
                        self.dependencies
                            .entry(symbol.id.clone())
                            .or_default()
                            .push(Dependency {
                                from: symbol.id.clone(),
                                to: target_id.clone(),
                                kind: reference.kind,
                            });
                    }
                }
            }

            symbol.dependencies = deps;
        }
    }

    /// Check if path should be ignored (synchronous - uses try_lock to avoid blocking)
    fn should_ignore(&self, path: &Path) -> bool {
        let is_dir = path.is_dir();

        // Check ignore matcher (gitignore + default patterns) using try_lock
        if let Ok(matcher) = self.ignore_matcher.try_lock() {
            if matcher.should_ignore(path, is_dir) {
                return true;
            }
        }

        // Check config ignores
        let path_str = path.to_string_lossy();
        for ignore in &self.config.ignore {
            if path_str.contains(ignore) {
                return true;
            }
        }

        false
    }

    /// Walk directory and index all files
    async fn walk_and_index(&mut self, root: &Path) -> Result<()> {
        use tokio::fs;

        let mut entries = fs::read_dir(root).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                if !self.should_ignore(&path) {
                    Box::pin(self.walk_and_index(&path)).await?;
                }
            } else if path.is_file() {
                // Check if file has supported extension
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if self.config.languages.iter().any(|lang| {
                        matches!(
                            (lang.as_str(), ext),
                            ("rust", "rs")
                                | ("typescript", "ts" | "tsx")
                                | ("javascript", "js" | "jsx")
                                | ("python", "py")
                                | ("go", "go")
                        )
                    }) {
                        match self.index_file(&path).await {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!("Failed to index {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Walk directory and index only changed files (incremental)
    pub async fn walk_and_index_incremental(&mut self, root: &Path) -> Result<(usize, usize)> {
        use tokio::fs;

        let mut total_files = 0;
        let mut indexed_files = 0;

        let mut entries = fs::read_dir(root).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                if !self.should_ignore(&path) {
                    let (sub_total, sub_indexed) = Box::pin(self.walk_and_index_incremental(&path)).await?;
                    total_files += sub_total;
                    indexed_files += sub_indexed;
                }
            } else if path.is_file() {
                // Check if file has supported extension
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if self.config.languages.iter().any(|lang| {
                        matches!(
                            (lang.as_str(), ext),
                            ("rust", "rs")
                                | ("typescript", "ts" | "tsx")
                                | ("javascript", "js" | "jsx")
                                | ("python", "py")
                                | ("go", "go")
                        )
                    }) {
                        total_files += 1;
                        match self.index_file_incremental(&path).await {
                            Ok(true) => {
                                indexed_files += 1;
                            }
                            Ok(false) => {
                                // File unchanged, skipped
                            }
                            Err(e) => {
                                tracing::warn!("Failed to index {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok((total_files, indexed_files))
    }

    /// Get full definition of a symbol including body
    pub async fn get_definition(
        &self,
        symbol_id: &SymbolId,
        include_body: bool,
        _include_references: bool,
        include_dependencies: bool,
    ) -> Result<Option<SymbolDefinition>> {
        let symbol = match self.symbols.get(symbol_id) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };

        let mut body = None;
        if include_body {
            // Get the source code for this symbol
            let file_path = PathBuf::from(&symbol.location.file);
            if let Some(source) = self.source_cache.get(&file_path) {
                let lines: Vec<&str> = source.lines().collect();
                let start = (symbol.location.line_start - 1).min(lines.len());
                let end = symbol.location.line_end.min(lines.len());
                if start < end {
                    body = Some(lines[start..end].join("\n"));
                }
            } else {
                // Try to load from file if not in cache
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    let lines: Vec<&str> = content.lines().collect();
                    let start = (symbol.location.line_start - 1).min(lines.len());
                    let end = symbol.location.line_end.min(lines.len());
                    if start < end {
                        body = Some(lines[start..end].join("\n"));
                    }
                    // Cache it
                    self.source_cache.insert(file_path, content);
                }
            }
        }

        let mut dependencies = Vec::new();
        if include_dependencies {
            for dep_id in &symbol.dependencies {
                if let Some(dep_symbol) = self.symbols.get(dep_id) {
                    dependencies.push(dep_symbol.clone());
                }
            }
        }

        Ok(Some(SymbolDefinition {
            symbol,
            body,
            dependencies,
        }))
    }

    /// Find all references to a symbol
    pub async fn find_references(&self, symbol_id: &SymbolId) -> Result<Vec<Reference>> {
        let mut references = Vec::new();

        // Look through all symbols to find references to this one
        for entry in self.symbols.iter() {
            let symbol = entry.value();
            for reference in &symbol.references {
                if &reference.symbol_id == symbol_id {
                    references.push(reference.clone());
                }
            }
        }

        Ok(references)
    }

    /// Build dependency graph from a symbol
    pub async fn get_dependencies(
        &self,
        entry_point: &SymbolId,
        depth: Option<usize>,
        direction: DependencyDirection,
    ) -> Result<DependencyGraph> {
        let max_depth = depth.unwrap_or(10);
        let mut nodes = HashSet::new();
        let mut edges = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![(entry_point.clone(), 0)];

        while let Some((current_id, current_depth)) = queue.pop() {
            if current_depth >= max_depth || visited.contains(&current_id) {
                continue;
            }

            visited.insert(current_id.clone());
            nodes.insert(current_id.clone());

            match direction {
                DependencyDirection::Imports => {
                    // Follow dependencies (what this symbol imports/uses)
                    if let Some(deps) = self.dependencies.get(&current_id) {
                        for dep in deps.iter() {
                            edges.push((dep.from.clone(), dep.to.clone(), dep.kind));
                            queue.push((dep.to.clone(), current_depth + 1));
                        }
                    }
                }
                DependencyDirection::Exports => {
                    // Follow reverse dependencies (what uses this symbol)
                    for entry in self.dependencies.iter() {
                        for dep in entry.value().iter() {
                            if dep.to == current_id {
                                edges.push((dep.from.clone(), dep.to.clone(), dep.kind));
                                queue.push((dep.from.clone(), current_depth + 1));
                            }
                        }
                    }
                }
                DependencyDirection::Both => {
                    // Both directions
                    if let Some(deps) = self.dependencies.get(&current_id) {
                        for dep in deps.iter() {
                            edges.push((dep.from.clone(), dep.to.clone(), dep.kind));
                            queue.push((dep.to.clone(), current_depth + 1));
                        }
                    }

                    for entry in self.dependencies.iter() {
                        for dep in entry.value().iter() {
                            if dep.to == current_id {
                                edges.push((dep.from.clone(), dep.to.clone(), dep.kind));
                                queue.push((dep.from.clone(), current_depth + 1));
                            }
                        }
                    }
                }
            }
        }

        Ok(DependencyGraph {
            nodes: nodes.into_iter().collect(),
            edges,
        })
    }

    /// Filter symbols by detail level
    /// Public for testing purposes
    pub fn apply_detail_level(&self, symbol: &CodeSymbol, level: DetailLevel) -> CodeSymbol {
        let mut filtered = symbol.clone();

        match level {
            DetailLevel::Skeleton => {
                // Only keep name and signature (no body, no deps, no refs)
                // Signature is already minimal (just function name + params + return type)
                filtered.references = Vec::new();
                filtered.dependencies = Vec::new();

                // Clear doc comment for skeleton
                filtered.metadata.doc_comment = None;

                // Reduce token cost estimate significantly (skeleton is ~10-20% of full)
                filtered.metadata.token_cost = TokenCount::new(
                    (symbol.metadata.token_cost.0 as f32 * 0.15) as u32
                );
            }
            DetailLevel::Interface => {
                // Keep public interface: signature + first line of doc
                filtered.references = Vec::new();

                // Keep only first line of doc comment (if exists)
                if let Some(ref doc) = filtered.metadata.doc_comment {
                    let first_line = doc.lines().next().unwrap_or("");
                    if !first_line.is_empty() {
                        filtered.metadata.doc_comment = Some(first_line.to_string());
                    }
                }

                // Interface level is ~30-40% of full token cost
                filtered.metadata.token_cost = TokenCount::new(
                    (symbol.metadata.token_cost.0 as f32 * 0.35) as u32
                );
            }
            DetailLevel::Implementation => {
                // Keep implementation but not all metadata
                // Signature + full doc + body (but no references)
                filtered.references = Vec::new();

                // Implementation is ~60-70% of full
                filtered.metadata.token_cost = TokenCount::new(
                    (symbol.metadata.token_cost.0 as f32 * 0.65) as u32
                );
            }
            DetailLevel::Full => {
                // Keep everything as-is
                // No modifications needed
            }
        }

        filtered
    }

    /// Perform semantic search using vector embeddings
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<CodeSymbol>> {
        // Generate embedding for the query
        let query_embedding = self.embedding_engine.lock().generate_embedding(query)?;

        // Calculate cosine similarity for all symbols with embeddings
        let mut scored_symbols: Vec<(CodeSymbol, f32)> = Vec::new();

        for entry in self.symbols.iter() {
            let symbol = entry.value();

            if let Some(ref embedding) = symbol.embedding {
                let similarity = EmbeddingEngine::cosine_similarity(&query_embedding, embedding);
                scored_symbols.push((symbol.clone(), similarity));
            }
        }

        // Sort by similarity score (highest first)
        scored_symbols.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N results
        let results: Vec<CodeSymbol> = scored_symbols
            .into_iter()
            .take(limit)
            .map(|(symbol, _score)| symbol)
            .collect();

        Ok(results)
    }

    /// Hybrid search combining text-based and semantic search
    pub async fn hybrid_search(&self, query: &Query) -> Result<QueryResult> {
        let limit = query.max_results.unwrap_or(10);

        // Get text-based search results
        let text_results = self.search_symbols(query).await?;

        // Get semantic search results
        let semantic_results = self.semantic_search(&query.text, limit).await?;

        // Combine and deduplicate results
        let mut seen_ids = HashSet::new();
        let mut combined_symbols = Vec::new();
        let mut total_tokens = TokenCount::zero();

        // Add text results first (they are more precise)
        for symbol in text_results.symbols {
            if !seen_ids.contains(&symbol.id) {
                seen_ids.insert(symbol.id.clone());

                // Apply filters
                if let Some(ref types) = query.symbol_types {
                    if !types.contains(&symbol.kind) {
                        continue;
                    }
                }

                if let Some(ref scope) = query.scope {
                    if !symbol.location.file.starts_with(scope) {
                        continue;
                    }
                }

                // Check token budget
                if let Some(max_tokens) = query.max_tokens {
                    if total_tokens.0 + symbol.metadata.token_cost.0 > max_tokens.0 {
                        break;
                    }
                }

                total_tokens.add(symbol.metadata.token_cost);
                combined_symbols.push(symbol);

                if combined_symbols.len() >= limit {
                    break;
                }
            }
        }

        // Add semantic results to fill remaining slots
        for symbol in semantic_results {
            if combined_symbols.len() >= limit {
                break;
            }

            if !seen_ids.contains(&symbol.id) {
                seen_ids.insert(symbol.id.clone());

                // Apply filters
                if let Some(ref types) = query.symbol_types {
                    if !types.contains(&symbol.kind) {
                        continue;
                    }
                }

                if let Some(ref scope) = query.scope {
                    if !symbol.location.file.starts_with(scope) {
                        continue;
                    }
                }

                // Apply detail level
                let filtered = self.apply_detail_level(&symbol, query.detail_level);

                // Check token budget
                if let Some(max_tokens) = query.max_tokens {
                    if total_tokens.0 + filtered.metadata.token_cost.0 > max_tokens.0 {
                        break;
                    }
                }

                total_tokens.add(filtered.metadata.token_cost);
                combined_symbols.push(filtered);
            }
        }

        let truncated = query.max_tokens.is_some_and(|max| total_tokens > max)
            || combined_symbols.len() >= limit;

        Ok(QueryResult {
            symbols: combined_symbols,
            total_tokens,
            truncated,
            total_matches: None,
            offset: None,
            has_more: None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DependencyDirection {
    Imports,
    Exports,
    Both,
}

#[async_trait::async_trait]
impl Indexer for CodeIndexer {
    async fn index_project(&mut self, path: &Path, force: bool) -> Result<()> {
        use super::MonorepoConfig;

        if force {
            self.symbols.clear();
            self.name_index.clear();
            self.file_index.clear();
            self.dependencies.clear();
            self.source_cache.clear();
        }

        tracing::info!("Indexing project at {:?}", path);

        // Load .gitignore if it exists (read file first, then update matcher)
        let gitignore_path = path.join(".gitignore");
        if gitignore_path.exists() {
            match tokio::fs::read_to_string(&gitignore_path).await {
                Ok(content) => {
                    let mut matcher = self.ignore_matcher.lock().await;
                    matcher.load_gitignore_from_string(&content);
                }
                Err(e) => {
                    tracing::warn!("Failed to read .gitignore: {}", e);
                }
            }
        }

        // Detect monorepo configuration
        let monorepo_config = MonorepoConfig::detect(path).await?;

        let start_time = std::time::Instant::now();
        let initial_symbols = self.symbols.len();

        match monorepo_config.monorepo_type {
            super::MonorepoType::None => {
                // Single project - index normally
                tracing::info!("Indexing as single project");
                self.walk_and_index(path).await?;
            }
            _ => {
                // Monorepo - index all workspace directories
                tracing::info!(
                    "Detected monorepo ({:?}) with {} workspace(s)",
                    monorepo_config.monorepo_type,
                    monorepo_config.workspace_dirs.len()
                );

                if monorepo_config.workspace_dirs.is_empty() {
                    tracing::warn!("No workspace directories found, falling back to root indexing");
                    self.walk_and_index(path).await?;
                } else {
                    // Index each workspace directory
                    for (i, workspace_dir) in monorepo_config.workspace_dirs.iter().enumerate() {
                        tracing::info!(
                            "Indexing workspace {}/{}: {:?}",
                            i + 1,
                            monorepo_config.workspace_dirs.len(),
                            workspace_dir
                        );

                        if workspace_dir.exists() {
                            self.walk_and_index(workspace_dir).await?;
                        } else {
                            tracing::warn!("Workspace directory does not exist: {:?}", workspace_dir);
                        }
                    }
                }
            }
        }

        let elapsed = start_time.elapsed();
        let new_symbols = self.symbols.len() - initial_symbols;

        tracing::info!(
            "Indexing complete: {} total symbols ({} new) in {:.2}s",
            self.symbols.len(),
            new_symbols,
            elapsed.as_secs_f64()
        );

        Ok(())
    }

    async fn search_symbols(&self, query: &Query) -> Result<QueryResult> {
        // Check cache first (skip cache for token budget queries as results vary)
        if query.max_tokens.is_none() {
            let cache_key = SearchCacheKey::from_query(query);

            // Try to get from multi-level cache (checks L1 → L2 → L3)
            if let Some(cached_result) = self.search_cache.get(&cache_key).await? {
                tracing::debug!("Multi-level cache hit for query: {}", query.text);
                return Ok(cached_result);
            }
        }

        let mut all_matches = Vec::new();

        // First, try exact name match
        if let Some(symbol_ids) = self.name_index.get(&query.text) {
            for symbol_id in symbol_ids.iter() {
                if let Some(symbol) = self.symbols.get(symbol_id) {
                    // Apply filters
                    if let Some(ref types) = query.symbol_types {
                        if !types.contains(&symbol.kind) {
                            continue;
                        }
                    }

                    if let Some(ref scope) = query.scope {
                        if !symbol.location.file.starts_with(scope) {
                            continue;
                        }
                    }

                    // Apply detail level
                    #[allow(clippy::needless_borrow)]
                    let filtered = self.apply_detail_level(&symbol, query.detail_level);
                    all_matches.push(filtered);
                }
            }
        }

        // If no exact matches, do fuzzy search
        if all_matches.is_empty() {
            let query_lower = query.text.to_lowercase();

            for entry in self.symbols.iter() {
                let symbol = entry.value();

                // Check if query matches name or signature
                let name_lower = symbol.name.to_lowercase();
                let sig_lower = symbol.signature.to_lowercase();

                if !name_lower.contains(&query_lower) && !sig_lower.contains(&query_lower) {
                    continue;
                }

                // Apply filters
                if let Some(ref types) = query.symbol_types {
                    if !types.contains(&symbol.kind) {
                        continue;
                    }
                }

                if let Some(ref scope) = query.scope {
                    if !symbol.location.file.starts_with(scope) {
                        continue;
                    }
                }

                // Apply detail level
                #[allow(clippy::needless_borrow)]
                let filtered = self.apply_detail_level(&symbol, query.detail_level);
                all_matches.push(filtered);
            }
        }

        // Store total count before pagination
        let total_matches = all_matches.len();
        let offset = query.offset.unwrap_or(0);

        // Apply pagination
        let paginated_symbols: Vec<CodeSymbol> = all_matches
            .into_iter()
            .skip(offset)
            .take(query.max_results.unwrap_or(20))
            .collect();

        // Apply token budget if specified
        let mut final_symbols = Vec::new();
        let mut total_tokens = TokenCount::zero();
        let mut truncated_by_tokens = false;

        for symbol in paginated_symbols {
            if let Some(max_tokens) = query.max_tokens {
                if total_tokens.0 + symbol.metadata.token_cost.0 > max_tokens.0 {
                    truncated_by_tokens = true;
                    break;
                }
            }

            total_tokens.add(symbol.metadata.token_cost);
            final_symbols.push(symbol);
        }

        let has_more = offset + final_symbols.len() < total_matches;
        let truncated = truncated_by_tokens || has_more;

        let result = QueryResult {
            symbols: final_symbols,
            total_tokens,
            truncated,
            total_matches: Some(total_matches),
            offset: Some(offset),
            has_more: Some(has_more),
        };

        // Store in multi-level cache (skip if token budget was used as results vary)
        if query.max_tokens.is_none() {
            let cache_key = SearchCacheKey::from_query(query);
            self.search_cache.put(cache_key, result.clone()).await?;
            tracing::debug!("Stored search result in multi-level cache for query: {}", query.text);
        }

        Ok(result)
    }

    async fn get_symbol(&self, id: &str) -> Result<Option<CodeSymbol>> {
        let symbol_id = SymbolId::new(id);
        Ok(self.symbols.get(&symbol_id).map(|s| s.clone()))
    }

    async fn update_file(&mut self, path: &Path) -> Result<()> {
        // Remove old symbols for this file
        if let Some(old_symbols) = self.file_index.get(path) {
            for symbol_id in old_symbols.iter() {
                self.symbols.remove(symbol_id);

                // Remove from name index
                for mut entry in self.name_index.iter_mut() {
                    entry.value_mut().retain(|id| id != symbol_id);
                }

                // Remove from dependencies
                self.dependencies.remove(symbol_id);
            }
        }
        self.file_index.remove(path);

        // Clear source cache
        self.source_cache.remove(path);

        // Invalidate multi-level search cache on file changes
        // Note: We clear the entire cache since we can't efficiently track which
        // cache entries are affected by this file update
        self.search_cache.clear().await?;
        tracing::debug!("Multi-level search cache cleared due to file update: {:?}", path);

        // Re-index the file
        self.index_file(path).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IndexConfig;
    use crate::storage::MemoryStorage;
    use crate::types::SymbolKind;
    use tempfile::TempDir;

    async fn setup_test_indexer() -> (CodeIndexer, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MemoryStorage::new());
        let config = IndexConfig {
            languages: vec!["rust".to_string(), "typescript".to_string()],
            ignore: vec!["target".to_string(), "node_modules".to_string()],
            max_file_size: "1MB".to_string(),
        };

        let indexer = CodeIndexer::new(storage, config).unwrap();
        (indexer, temp_dir)
    }

    #[tokio::test]
    async fn test_index_and_search() {
        let (mut indexer, _temp) = setup_test_indexer().await;

        // Create a test file
        let test_dir = TempDir::new().unwrap();
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(
            &test_file,
            r#"
            pub fn test_function(x: i32) -> i32 {
                x + 1
            }

            pub struct TestStruct {
                field: i32,
            }
            "#,
        )
        .await
        .unwrap();

        // Index the directory
        indexer.index_project(test_dir.path(), false).await.unwrap();

        // Search for function
        let query = Query::new("test_function".to_string());
        let result = indexer.search_symbols(&query).await.unwrap();

        assert!(!result.symbols.is_empty());
        assert!(result
            .symbols
            .iter()
            .any(|s| s.name == "test_function" && s.kind == SymbolKind::Function));
    }

    #[tokio::test]
    async fn test_get_definition() {
        let (mut indexer, _temp) = setup_test_indexer().await;

        // Create and index a test file
        let test_dir = TempDir::new().unwrap();
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(
            &test_file,
            r#"
            pub fn test_function(x: i32) -> i32 {
                x + 1
            }
            "#,
        )
        .await
        .unwrap();

        indexer.index_project(test_dir.path(), false).await.unwrap();

        // Find the symbol
        let query = Query::new("test_function".to_string());
        let result = indexer.search_symbols(&query).await.unwrap();
        let symbol = result.symbols.first().unwrap();

        // Get full definition
        let definition = indexer
            .get_definition(&symbol.id, true, false, false)
            .await
            .unwrap()
            .unwrap();

        assert!(definition.body.is_some());
        assert!(definition.body.unwrap().contains("x + 1"));
    }

    #[tokio::test]
    async fn test_dependency_graph() {
        let (mut indexer, _temp) = setup_test_indexer().await;

        // Create test file with dependencies
        let test_dir = TempDir::new().unwrap();
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(
            &test_file,
            r#"
            pub struct User {
                name: String,
            }

            pub fn create_user(name: String) -> User {
                User { name }
            }
            "#,
        )
        .await
        .unwrap();

        indexer.index_project(test_dir.path(), false).await.unwrap();

        // Find create_user function
        let query = Query::new("create_user".to_string());
        let result = indexer.search_symbols(&query).await.unwrap();
        let symbol = result.symbols.first().unwrap();

        // Get dependencies
        let graph = indexer
            .get_dependencies(&symbol.id, Some(5), DependencyDirection::Both)
            .await
            .unwrap();

        assert!(!graph.nodes.is_empty());
    }
}
