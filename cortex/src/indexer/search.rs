use crate::types::{CodeSymbol, SymbolId};
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{FuzzyTermQuery, QueryParser, TermQuery};
use tantivy::schema::{Field, Schema, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, Term};

/// Full-text search engine powered by Tantivy
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Arc<parking_lot::Mutex<IndexWriter>>,
    schema: SearchSchema,
}

/// Search schema
struct SearchSchema {
    symbol_id: Field,
    name: Field,
    signature: Field,
    doc_comment: Field,
    file_path: Field,
    kind: Field,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        // Define fields - use STRING for exact matching (not tokenized)
        let symbol_id = schema_builder.add_text_field("symbol_id", STRING | STORED);
        let name = schema_builder.add_text_field("name", TEXT | STORED);
        let signature = schema_builder.add_text_field("signature", TEXT | STORED);
        let doc_comment = schema_builder.add_text_field("doc_comment", TEXT | STORED);
        let file_path = schema_builder.add_text_field("file_path", TEXT | STORED);
        let kind = schema_builder.add_text_field("kind", TEXT | STORED);

        let schema = schema_builder.build();

        // Always create directory if it doesn't exist
        std::fs::create_dir_all(index_path)?;

        // Create or open index
        let index = if index_path.join("meta.json").exists() {
            // Index exists, open it
            Index::open_in_dir(index_path)?
        } else {
            // Create new index
            Index::create_in_dir(index_path, schema.clone())?
        };

        // Create reader and writer
        let reader = index.reader()?;

        let writer = index.writer(50_000_000)?; // 50MB buffer

        Ok(Self {
            index,
            reader,
            writer: Arc::new(parking_lot::Mutex::new(writer)),
            schema: SearchSchema {
                symbol_id,
                name,
                signature,
                doc_comment,
                file_path,
                kind,
            },
        })
    }

    /// Index a symbol
    pub fn index_symbol(&self, symbol: &CodeSymbol) -> Result<()> {
        let writer = self.writer.lock();

        let doc = doc!(
            self.schema.symbol_id => symbol.id.0.clone(),
            self.schema.name => symbol.name.clone(),
            self.schema.signature => symbol.signature.clone(),
            self.schema.doc_comment => symbol.metadata.doc_comment.clone().unwrap_or_default(),
            self.schema.file_path => symbol.location.file.clone(),
            self.schema.kind => symbol.kind.as_str(),
        );

        writer.add_document(doc)?;
        Ok(())
    }

    /// Index multiple symbols
    pub fn index_symbols(&self, symbols: &[CodeSymbol]) -> Result<()> {
        let writer = self.writer.lock();

        for symbol in symbols {
            let doc = doc!(
                self.schema.symbol_id => symbol.id.0.clone(),
                self.schema.name => symbol.name.clone(),
                self.schema.signature => symbol.signature.clone(),
                self.schema.doc_comment => symbol.metadata.doc_comment.clone().unwrap_or_default(),
                self.schema.file_path => symbol.location.file.clone(),
                self.schema.kind => symbol.kind.as_str(),
            );

            writer.add_document(doc)?;
        }

        Ok(())
    }

    /// Commit changes
    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock();
        writer.commit()?;
        Ok(())
    }

    /// Search for symbols by text query
    pub fn search(&self, query_text: &str, limit: usize) -> Result<Vec<SymbolId>> {
        let searcher = self.reader.searcher();

        // Create query parser for multiple fields
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.schema.name,
                self.schema.signature,
                self.schema.doc_comment,
            ],
        );

        let query = query_parser
            .parse_query(query_text)
            .context("Failed to parse query")?;

        // Search
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut symbol_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_field) = retrieved_doc.get_first(self.schema.symbol_id) {
                match id_field.as_str() {
                    Some(id_text) => {
                        symbol_ids.push(SymbolId::new(id_text));
                    }
                    None => continue,
                }
            }
        }

        Ok(symbol_ids)
    }

    /// Fuzzy search for symbols
    pub fn fuzzy_search(&self, query_text: &str, limit: usize) -> Result<Vec<SymbolId>> {
        let searcher = self.reader.searcher();

        // Create fuzzy queries for name field
        let name_term = Term::from_field_text(self.schema.name, query_text);
        let name_query = FuzzyTermQuery::new(name_term, 2, true); // max distance = 2

        // Search
        let top_docs = searcher.search(&name_query, &TopDocs::with_limit(limit))?;

        let mut symbol_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_field) = retrieved_doc.get_first(self.schema.symbol_id) {
                match id_field.as_str() {
                    Some(id_text) => {
                        symbol_ids.push(SymbolId::new(id_text));
                    }
                    None => continue,
                }
            }
        }

        Ok(symbol_ids)
    }

    /// Search by file path
    pub fn search_by_file(&self, file_path: &str, limit: usize) -> Result<Vec<SymbolId>> {
        let searcher = self.reader.searcher();

        let term = Term::from_field_text(self.schema.file_path, file_path);
        let query = TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut symbol_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_field) = retrieved_doc.get_first(self.schema.symbol_id) {
                match id_field.as_str() {
                    Some(id_text) => {
                        symbol_ids.push(SymbolId::new(id_text));
                    }
                    None => continue,
                }
            }
        }

        Ok(symbol_ids)
    }

    /// Delete a symbol from the index
    pub fn delete_symbol(&self, symbol_id: &SymbolId) -> Result<()> {
        let mut writer = self.writer.lock();
        let term = Term::from_field_text(self.schema.symbol_id, &symbol_id.0);
        writer.delete_term(term);
        writer.commit()?;
        Ok(())
    }

    /// Clear the entire index
    pub fn clear(&self) -> Result<()> {
        let mut writer = self.writer.lock();
        writer.delete_all_documents()?;
        writer.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        CodeSymbol, Hash, Location, SymbolKind, SymbolMetadata, TokenCount,
    };
    use tempfile::TempDir;

    fn create_test_symbol(name: &str, kind: SymbolKind) -> CodeSymbol {
        CodeSymbol {
            id: SymbolId::generate(),
            name: name.to_string(),
            kind,
            signature: format!("fn {}()", name),
            body_hash: Hash::from_string("test"),
            location: Location::new("test.rs".to_string(), 1, 10, 0, 0),
            references: Vec::new(),
            dependencies: Vec::new(),
            metadata: SymbolMetadata {
                complexity: 1,
                token_cost: TokenCount::new(100),
                last_modified: None,
                authors: Vec::new(),
                doc_comment: Some(format!("Documentation for {}", name)),
                test_coverage: 0.0,
                usage_frequency: 0,
            },
            embedding: None,
        }
    }

    #[test]
    fn test_index_and_search() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("index");
        let engine = SearchEngine::new(&index_dir).unwrap();

        // Index some symbols
        let symbol1 = create_test_symbol("test_function", SymbolKind::Function);
        let symbol2 = create_test_symbol("another_test", SymbolKind::Function);
        let symbol3 = create_test_symbol("TestStruct", SymbolKind::Struct);

        engine.index_symbol(&symbol1).unwrap();
        engine.index_symbol(&symbol2).unwrap();
        engine.index_symbol(&symbol3).unwrap();
        engine.commit().unwrap();

        // Reload reader to see committed changes
        engine.reader.reload().unwrap();

        // Search
        let results = engine.search("test", 10).unwrap();
        assert!(!results.is_empty());
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_fuzzy_search() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("index");
        let engine = SearchEngine::new(&index_dir).unwrap();

        let symbol = create_test_symbol("myfunction", SymbolKind::Function);
        engine.index_symbol(&symbol).unwrap();
        engine.commit().unwrap();

        // Reload reader to see committed changes
        engine.reader.reload().unwrap();

        // Fuzzy search with typo (should match "myfunction")
        let results = engine.fuzzy_search("myfunktion", 10).unwrap();
        assert!(!results.is_empty(), "Fuzzy search should find 'myfunction' when searching for 'myfunktion'");
    }

    #[test]
    fn test_delete_symbol() {
        let temp_dir = TempDir::new().unwrap();
        let index_dir = temp_dir.path().join("index");
        let engine = SearchEngine::new(&index_dir).unwrap();

        let symbol = create_test_symbol("deletetest", SymbolKind::Function);
        let symbol_id = symbol.id.clone();

        engine.index_symbol(&symbol).unwrap();
        engine.commit().unwrap();

        // Reload reader to see committed changes
        engine.reader.reload().unwrap();

        // Verify it exists
        let results = engine.search("deletetest", 10).unwrap();
        assert_eq!(results.len(), 1, "Symbol should exist before deletion");
        assert_eq!(results[0], symbol_id, "Found symbol should match expected ID");

        // Delete it (delete_symbol now commits internally)
        engine.delete_symbol(&symbol_id).unwrap();

        // Reload reader to see committed changes
        engine.reader.reload().unwrap();

        // Verify it's gone
        let results = engine.search("deletetest", 10).unwrap();
        assert_eq!(results.len(), 0, "Symbol should not be found after deletion");
    }
}
