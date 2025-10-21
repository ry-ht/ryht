//! Comprehensive unit tests for cortex-core

use cortex_core::prelude::*;
use cortex_core::metadata::{MetadataBuilder, MetadataExtractor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_error_creation() {
    let err = CortexError::storage("test error");
    assert!(err.is_storage());
    assert!(!err.is_database());
    assert!(!err.is_not_found());
}

#[test]
fn test_database_error() {
    let err = CortexError::database("connection failed");
    assert!(err.is_database());
    assert!(!err.is_storage());
}

#[test]
fn test_not_found_error() {
    let err = CortexError::not_found("document", "123");
    assert!(err.is_not_found());
    assert!(!err.is_storage());
}

#[test]
fn test_error_display() {
    let err = CortexError::storage("test");
    assert_eq!(format!("{}", err), "Storage error: test");

    let err = CortexError::not_found("document", "123");
    assert_eq!(format!("{}", err), "Not found: document with id 123");
}

#[test]
fn test_all_error_constructors() {
    let errors = vec![
        CortexError::storage("storage"),
        CortexError::database("database"),
        CortexError::invalid_input("invalid"),
        CortexError::config("config"),
        CortexError::vfs("vfs"),
        CortexError::memory("memory"),
        CortexError::ingestion("ingestion"),
        CortexError::mcp("mcp"),
        CortexError::concurrency("concurrency"),
        CortexError::timeout("timeout"),
        CortexError::internal("internal"),
    ];

    // All errors should be created successfully
    assert_eq!(errors.len(), 11);
}

#[test]
fn test_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let cortex_err: CortexError = io_err.into();
    assert!(matches!(cortex_err, CortexError::Io(_)));
}

#[test]
fn test_error_from_serde_json() {
    let json_err = serde_json::from_str::<HashMap<String, String>>("invalid json").unwrap_err();
    let cortex_err: CortexError = json_err.into();
    assert!(matches!(cortex_err, CortexError::Serialization(_)));
}

// ============================================================================
// ID Tests
// ============================================================================

#[test]
fn test_id_uniqueness() {
    let ids: Vec<CortexId> = (0..100).map(|_| CortexId::new()).collect();
    let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 100);
}

#[test]
fn test_id_default() {
    let id1 = CortexId::default();
    let id2 = CortexId::default();
    assert_ne!(id1, id2);
}

#[test]
fn test_id_display() {
    let id = CortexId::new();
    let s = format!("{}", id);
    assert_eq!(s.len(), 36); // UUID format
}

#[test]
fn test_id_parse_invalid() {
    let result = CortexId::parse("invalid-uuid");
    assert!(result.is_err());
}

#[test]
fn test_id_from_str() {
    let id = CortexId::new();
    let s = id.to_string();
    let parsed: CortexId = s.parse().unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn test_id_uuid_conversion() {
    let uuid = uuid::Uuid::new_v4();
    let id = CortexId::from_uuid(uuid);
    assert_eq!(id.as_uuid(), &uuid);

    let uuid_back: uuid::Uuid = id.into();
    assert_eq!(uuid, uuid_back);
}

#[test]
fn test_id_hash() {
    let mut map = HashMap::new();
    let id = CortexId::new();
    map.insert(id, "value");
    assert_eq!(map.get(&id), Some(&"value"));
}

// ============================================================================
// Types Tests
// ============================================================================

#[test]
fn test_project_creation() {
    let project = Project::new("test".to_string(), PathBuf::from("/test"));
    assert_eq!(project.name, "test");
    assert_eq!(project.path, PathBuf::from("/test"));
    assert!(project.description.is_none());
    assert!(project.metadata.is_empty());
}

#[test]
fn test_project_serialization() {
    let project = Project::new("test".to_string(), PathBuf::from("/test"));
    let json = serde_json::to_string(&project).unwrap();
    let deserialized: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(project, deserialized);
}

#[test]
fn test_search_query_builder() {
    let query = SearchQuery::new("test query".to_string())
        .with_limit(20)
        .with_threshold(0.8)
        .with_filter("type".to_string(), "document".to_string());

    assert_eq!(query.query, "test query");
    assert_eq!(query.limit, 20);
    assert_eq!(query.threshold, Some(0.8));
    assert_eq!(query.filters.get("type"), Some(&"document".to_string()));
}

#[test]
fn test_search_query_default() {
    let query = SearchQuery::new("test".to_string());
    assert_eq!(query.limit, 10);
    assert!(query.threshold.is_none());
    assert!(query.filters.is_empty());
}

#[test]
fn test_entity_types() {
    let types = vec![
        EntityType::Document,
        EntityType::Chunk,
        EntityType::Symbol,
        EntityType::Episode,
    ];

    for entity_type in types {
        let json = serde_json::to_string(&entity_type).unwrap();
        let deserialized: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(entity_type, deserialized);
    }
}

#[test]
fn test_symbol_kinds() {
    let kinds = vec![
        SymbolKind::Function,
        SymbolKind::Method,
        SymbolKind::Class,
        SymbolKind::Struct,
        SymbolKind::Enum,
        SymbolKind::Interface,
        SymbolKind::Trait,
        SymbolKind::Type,
        SymbolKind::Constant,
        SymbolKind::Variable,
        SymbolKind::Module,
        SymbolKind::Namespace,
    ];

    for kind in kinds {
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: SymbolKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, deserialized);
    }
}

#[test]
fn test_relation_types() {
    let types = vec![
        RelationType::Contains,
        RelationType::References,
        RelationType::Imports,
        RelationType::Extends,
        RelationType::Implements,
        RelationType::Calls,
        RelationType::DependsOn,
        RelationType::SimilarTo,
        RelationType::PartOf,
    ];

    for rel_type in types {
        let json = serde_json::to_string(&rel_type).unwrap();
        let deserialized: RelationType = serde_json::from_str(&json).unwrap();
        assert_eq!(rel_type, deserialized);
    }
}

#[test]
fn test_range() {
    let range = Range {
        start_line: 10,
        start_column: 5,
        end_line: 20,
        end_column: 10,
    };

    let json = serde_json::to_string(&range).unwrap();
    let deserialized: Range = serde_json::from_str(&json).unwrap();
    assert_eq!(range, deserialized);
}

#[test]
fn test_chunk_creation() {
    let chunk = Chunk {
        id: CortexId::new(),
        document_id: CortexId::new(),
        content: "test content".to_string(),
        start_offset: 0,
        end_offset: 12,
        chunk_index: 0,
        metadata: HashMap::new(),
    };

    assert_eq!(chunk.content, "test content");
    assert_eq!(chunk.chunk_index, 0);
}

#[test]
fn test_embedding_creation() {
    let embedding = Embedding {
        id: CortexId::new(),
        entity_id: CortexId::new(),
        entity_type: EntityType::Chunk,
        vector: vec![0.1, 0.2, 0.3],
        model: "test-model".to_string(),
        created_at: chrono::Utc::now(),
    };

    assert_eq!(embedding.vector.len(), 3);
    assert_eq!(embedding.model, "test-model");
    assert_eq!(embedding.entity_type, EntityType::Chunk);
}

#[test]
fn test_symbol_creation() {
    let symbol = Symbol {
        id: CortexId::new(),
        document_id: CortexId::new(),
        name: "test_function".to_string(),
        kind: SymbolKind::Function,
        range: Range {
            start_line: 1,
            start_column: 0,
            end_line: 10,
            end_column: 1,
        },
        signature: Some("fn test_function()".to_string()),
        documentation: Some("Test function".to_string()),
        metadata: HashMap::new(),
    };

    assert_eq!(symbol.name, "test_function");
    assert_eq!(symbol.kind, SymbolKind::Function);
    assert!(symbol.signature.is_some());
}

#[test]
fn test_relation_creation() {
    let relation = Relation {
        id: CortexId::new(),
        source_id: CortexId::new(),
        target_id: CortexId::new(),
        relation_type: RelationType::Calls,
        weight: 0.9,
        metadata: HashMap::new(),
    };

    assert_eq!(relation.relation_type, RelationType::Calls);
    assert_eq!(relation.weight, 0.9);
}

#[test]
fn test_episode_creation() {
    let episode = Episode {
        id: CortexId::new(),
        project_id: CortexId::new(),
        session_id: Some("session-123".to_string()),
        content: "test episode".to_string(),
        context: HashMap::new(),
        importance: 0.8,
        created_at: chrono::Utc::now(),
        accessed_count: 0,
        last_accessed_at: None,
    };

    assert_eq!(episode.content, "test episode");
    assert_eq!(episode.importance, 0.8);
    assert_eq!(episode.accessed_count, 0);
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[test]
fn test_metadata_builder_basic() {
    let metadata = MetadataBuilder::new()
        .add("key1", "value1")
        .add("key2", "value2")
        .build();

    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata.get("key1"), Some(&"value1".to_string()));
    assert_eq!(metadata.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_metadata_builder_empty() {
    let metadata = MetadataBuilder::new().build();
    assert!(metadata.is_empty());
}

#[test]
fn test_metadata_builder_option_some() {
    let metadata = MetadataBuilder::new()
        .add_option("key1", Some("value1"))
        .build();

    assert_eq!(metadata.get("key1"), Some(&"value1".to_string()));
}

#[test]
fn test_metadata_builder_option_none() {
    let metadata = MetadataBuilder::new()
        .add_option("key1", None::<String>)
        .build();

    assert!(metadata.is_empty());
}

#[test]
fn test_metadata_extractor_from_path() {
    let path = Path::new("/test/dir/file.rs");
    let metadata = MetadataExtractor::from_path(path);

    assert_eq!(metadata.get("extension"), Some(&"rs".to_string()));
    assert_eq!(metadata.get("filename"), Some(&"file.rs".to_string()));
}

#[test]
fn test_metadata_extractor_path_no_extension() {
    let path = Path::new("/test/dir/Makefile");
    let metadata = MetadataExtractor::from_path(path);

    assert_eq!(metadata.get("filename"), Some(&"Makefile".to_string()));
    assert!(metadata.get("extension").is_none());
}

#[test]
fn test_metadata_extractor_from_content() {
    let content = "line 1\nline 2\nline 3";
    let metadata = MetadataExtractor::from_content(content);

    assert_eq!(metadata.get("length"), Some(&"20".to_string()));
    assert_eq!(metadata.get("lines"), Some(&"3".to_string()));
}

#[test]
fn test_metadata_extractor_empty_content() {
    let content = "";
    let metadata = MetadataExtractor::from_content(content);

    assert_eq!(metadata.get("length"), Some(&"0".to_string()));
    assert_eq!(metadata.get("lines"), Some(&"0".to_string()));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_document_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("language".to_string(), "rust".to_string());
    metadata.insert("category".to_string(), "source".to_string());

    let document = Document {
        id: CortexId::new(),
        project_id: CortexId::new(),
        path: "/test/file.rs".to_string(),
        content_hash: "abc123".to_string(),
        size: 1024,
        mime_type: "text/x-rust".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        metadata,
    };

    assert_eq!(document.metadata.get("language"), Some(&"rust".to_string()));
    assert_eq!(document.size, 1024);
}

#[test]
fn test_search_result_serialization() {
    let chunk = Chunk {
        id: CortexId::new(),
        document_id: CortexId::new(),
        content: "test".to_string(),
        start_offset: 0,
        end_offset: 4,
        chunk_index: 0,
        metadata: HashMap::new(),
    };

    let result = SearchResult {
        item: chunk,
        score: 0.95,
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: SearchResult<Chunk> = serde_json::from_str(&json).unwrap();
    assert_eq!(result.score, deserialized.score);
}

#[test]
fn test_system_stats() {
    let stats = SystemStats {
        total_projects: 10,
        total_documents: 100,
        total_chunks: 1000,
        total_embeddings: 1000,
        total_episodes: 50,
        storage_size_bytes: 1024 * 1024,
        last_updated: chrono::Utc::now(),
    };

    assert_eq!(stats.total_projects, 10);
    assert_eq!(stats.storage_size_bytes, 1024 * 1024);
}

// ============================================================================
// Property-based Tests
// ============================================================================

#[cfg(feature = "proptest")]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_id_roundtrip_property(uuid_str in "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}") {
            if let Ok(id) = CortexId::parse(&uuid_str) {
                let s = id.to_string();
                let parsed = CortexId::parse(&s).unwrap();
                prop_assert_eq!(id, parsed);
            }
        }

        #[test]
        fn test_search_query_limit(limit in 1usize..=1000usize) {
            let query = SearchQuery::new("test".to_string()).with_limit(limit);
            prop_assert_eq!(query.limit, limit);
        }

        #[test]
        fn test_search_query_threshold(threshold in 0.0f32..=1.0f32) {
            let query = SearchQuery::new("test".to_string()).with_threshold(threshold);
            prop_assert_eq!(query.threshold, Some(threshold));
        }
    }
}
