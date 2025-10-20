# Cortex Semantic Search - System Architecture

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Cortex Applications                           │
│         (MCP Server, CLI, Dashboard, Agent Workflows)                │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Semantic Search Engine API                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐     │
│  │   Search     │  │    Index     │  │   Configuration       │     │
│  │   Methods    │  │   Methods    │  │   & Monitoring        │     │
│  └──────────────┘  └──────────────┘  └──────────────────────┘     │
└────────────────────────────┬────────────────────────────────────────┘
                             │
       ┌─────────────────────┼──────────────────────┐
       ▼                     ▼                      ▼
┌─────────────┐    ┌──────────────────┐    ┌───────────────┐
│   Query     │    │    Embedding      │    │    HNSW       │
│  Processor  │    │    Providers      │    │  Vector Index │
└─────────────┘    └──────────────────┘    └───────────────┘
       │                    │                        │
       ▼                    ▼                        ▼
┌─────────────┐    ┌──────────────────┐    ┌───────────────┐
│   Query     │    │   OpenAI API     │    │ In-Memory     │
│  Expander   │    │   ONNX Runtime   │    │ Index Store   │
│             │    │   Ollama Server  │    │               │
└─────────────┘    └──────────────────┘    └───────────────┘
       │                    │                        │
       ▼                    ▼                        ▼
┌─────────────┐    ┌──────────────────┐    ┌───────────────┐
│  Result     │    │   Embedding      │    │    Disk       │
│  Ranker     │    │     Cache        │    │ Persistence   │
└─────────────┘    └──────────────────┘    └───────────────┘
       │                    │                        │
       └────────────────────┴────────────────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │ Query Cache   │
                    └───────────────┘
```

## Component Interaction Flow

### 1. Document Indexing Flow

```
User/System
    │
    ├─ index_document(id, content, entity_type, metadata)
    │
    ▼
SemanticSearchEngine
    │
    ├─ Check cache for existing embedding
    │   │
    │   ├─ Cache HIT → Use cached embedding
    │   │
    │   └─ Cache MISS
    │       │
    │       ▼
    │   EmbeddingProvider (with fallback)
    │       │
    │       ├─ Try Primary (OpenAI)
    │       │   ├─ Success → Return embedding
    │       │   └─ Failure → Try fallback
    │       │
    │       ├─ Try Fallback #1 (ONNX)
    │       │   ├─ Success → Return embedding
    │       │   └─ Failure → Try next fallback
    │       │
    │       └─ Try Fallback #2 (Ollama)
    │           └─ Success/Failure
    │
    ├─ Cache embedding (if generated)
    │
    ├─ Create IndexedDocument
    │   └─ Store in documents HashMap
    │
    └─ Insert vector into HNSW Index
        └─ Update spatial index structure
```

### 2. Search Flow

```
User Query: "How to implement authentication?"
    │
    ▼
SemanticSearchEngine.search(query, limit)
    │
    ├─ Check Query Cache
    │   │
    │   ├─ Cache HIT → Return cached results (2ms)
    │   │
    │   └─ Cache MISS → Continue
    │
    ├─ QueryProcessor.process(query)
    │   │
    │   ├─ Normalize: "how to implement authentication?"
    │   │
    │   ├─ Detect Intent: Examples
    │   │
    │   ├─ Extract Keywords: ["implement", "authentication"]
    │   │
    │   ├─ Extract Filters: (none in this case)
    │   │
    │   └─ Expand Query:
    │       ├─ "how to implement authentication"
    │       ├─ "implement authentication"
    │       ├─ "authentication example"
    │       └─ "authentication usage"
    │
    ├─ Generate Query Embedding
    │   │
    │   ├─ Check Embedding Cache
    │   │   ├─ HIT → Use cached (0.1ms)
    │   │   └─ MISS → Generate (45ms)
    │   │
    │   └─ EmbeddingProvider.embed(normalized_query)
    │
    ├─ HNSW Index Search
    │   │
    │   ├─ Find k nearest neighbors (k = limit * 2)
    │   │   │
    │   │   ├─ Calculate cosine distances
    │   │   │
    │   │   ├─ Traverse HNSW graph
    │   │   │
    │   │   └─ Return top-k results with scores
    │   │
    │   └─ Results: [(doc_id, score), ...]
    │
    ├─ Apply Filters
    │   │
    │   ├─ Filter by entity_type (if specified)
    │   │
    │   ├─ Filter by metadata (if specified)
    │   │
    │   └─ Filter by min_score (if specified)
    │
    ├─ Convert to RankableDocuments
    │   │
    │   └─ Fetch document content and metadata
    │
    ├─ Result Ranking (if enabled)
    │   │
    │   ├─ Calculate semantic score (from HNSW)
    │   │
    │   ├─ Calculate keyword score (TF-IDF)
    │   │   └─ Count keyword occurrences
    │   │
    │   ├─ Calculate recency score (time decay)
    │   │   └─ Parse timestamp from metadata
    │   │
    │   ├─ Calculate popularity score
    │   │   └─ Parse views, references from metadata
    │   │
    │   ├─ Compute final score (weighted combination)
    │   │   └─ semantic*0.7 + keyword*0.2 + recency*0.05 + popularity*0.05
    │   │
    │   └─ Sort by final score (descending)
    │
    ├─ Apply Threshold and Limit
    │   │
    │   ├─ Filter results with score >= threshold
    │   │
    │   └─ Take top 'limit' results
    │
    ├─ Convert to SearchResults
    │
    ├─ Cache Results
    │
    └─ Return Vec<SearchResult>

Total Time: 67μs - 80ms (depending on cache state)
```

### 3. Batch Indexing Flow

```
User/System
    │
    ├─ index_batch(documents: Vec<(id, content, type, metadata)>)
    │
    ▼
SemanticSearchEngine
    │
    ├─ Extract all content texts
    │   └─ texts = [doc1.content, doc2.content, ...]
    │
    ├─ Generate embeddings in batch
    │   │
    │   └─ EmbeddingProvider.embed_batch(texts)
    │       │
    │       ├─ Batch API call (OpenAI: up to 2048 texts)
    │       │   └─ Single HTTP request for all texts
    │       │
    │       └─ Returns: Vec<Vector>
    │
    ├─ Create IndexedDocuments
    │   │
    │   └─ For each (doc, embedding):
    │       ├─ Store in documents HashMap
    │       └─ Prepare for batch insert
    │
    └─ Batch insert into HNSW Index
        │
        └─ index.insert_batch(items)
            └─ Efficient bulk insertion

Performance: ~38ms for 100 documents
```

## Data Flow Diagram

```
┌─────────────┐
│ Text Input  │
└──────┬──────┘
       │
       ▼
┌──────────────────────────────┐
│   Query Processor            │
│                              │
│  ┌────────────────────────┐ │
│  │ Normalize              │ │
│  │ • Lowercase            │ │
│  │ • Remove whitespace    │ │
│  └────────────────────────┘ │
│           │                  │
│           ▼                  │
│  ┌────────────────────────┐ │
│  │ Intent Detection       │ │
│  │ • Code, Docs, Examples │ │
│  └────────────────────────┘ │
│           │                  │
│           ▼                  │
│  ┌────────────────────────┐ │
│  │ Keyword Extraction     │ │
│  │ • Remove stop words    │ │
│  └────────────────────────┘ │
│           │                  │
│           ▼                  │
│  ┌────────────────────────┐ │
│  │ Query Expansion        │ │
│  │ • Synonyms             │ │
│  │ • Intent variations    │ │
│  └────────────────────────┘ │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│  Embedding Provider          │
│                              │
│  ┌────────────────────────┐ │
│  │ Check Cache            │ │
│  │ • Key: (text, model)   │ │
│  │ • TTL: 1 hour          │ │
│  └────────────────────────┘ │
│           │                  │
│     HIT   │   MISS           │
│       │   │   │              │
│       │   │   ▼              │
│       │   │ ┌──────────────┐│
│       │   │ │ Generate     ││
│       │   │ │ Embedding    ││
│       │   │ │              ││
│       │   │ │ OpenAI API   ││
│       │   │ │    or        ││
│       │   │ │ ONNX Local   ││
│       │   │ │    or        ││
│       │   │ │ Ollama       ││
│       │   │ └──────────────┘│
│       │   │   │              │
│       │   └───┼─ Cache      │
│       └───────┘              │
└──────────────┬───────────────┘
               │
               ▼ Vector[384]
┌──────────────────────────────┐
│  HNSW Vector Index           │
│                              │
│  Graph Structure:            │
│                              │
│  Layer 2: [○─○─○]           │
│            │ │ │             │
│  Layer 1: [○─○─○─○─○]       │
│            │││││││││         │
│  Layer 0: [○○○○○○○○○○○]     │
│                              │
│  • M=32 connections/node     │
│  • Cosine distance metric    │
│  • ef_search=100             │
│                              │
│  K-NN Search:                │
│  1. Start at entry point     │
│  2. Traverse top layer       │
│  3. Descend to lower layers  │
│  4. Find k nearest neighbors │
│                              │
└──────────────┬───────────────┘
               │
               ▼ Vec<(id, score)>
┌──────────────────────────────┐
│  Result Ranker               │
│                              │
│  ┌────────────────────────┐ │
│  │ Semantic Score (70%)   │ │
│  │ • From HNSW search     │ │
│  └────────────────────────┘ │
│           +                  │
│  ┌────────────────────────┐ │
│  │ Keyword Score (20%)    │ │
│  │ • TF-IDF calculation   │ │
│  └────────────────────────┘ │
│           +                  │
│  ┌────────────────────────┐ │
│  │ Recency Score (5%)     │ │
│  │ • Time decay function  │ │
│  └────────────────────────┘ │
│           +                  │
│  ┌────────────────────────┐ │
│  │ Popularity Score (5%)  │ │
│  │ • Views, references    │ │
│  └────────────────────────┘ │
│           =                  │
│  ┌────────────────────────┐ │
│  │ Final Score            │ │
│  │ • Weighted sum         │ │
│  └────────────────────────┘ │
│           │                  │
│           ▼                  │
│  ┌────────────────────────┐ │
│  │ Sort by Score (desc)   │ │
│  └────────────────────────┘ │
└──────────────┬───────────────┘
               │
               ▼ Vec<SearchResult>
┌──────────────────────────────┐
│  Query Cache                 │
│                              │
│  • Key: (query, limit)       │
│  • Value: Results            │
│  • TTL: 5 minutes            │
│  • Size: 1000 entries        │
└──────────────┬───────────────┘
               │
               ▼
         Search Results
```

## Storage Layout

```
Memory Layout:
┌─────────────────────────────────────────────┐
│ SemanticSearchEngine                        │
├─────────────────────────────────────────────┤
│                                             │
│  documents: DashMap<DocumentId, Doc>        │
│  ┌────────┬──────────────────────────────┐ │
│  │ "doc1" │ IndexedDocument {            │ │
│  │        │   content: "...",            │ │
│  │        │   embedding: Vec<f32>[384],  │ │
│  │        │   metadata: {...}            │ │
│  │        │ }                            │ │
│  └────────┴──────────────────────────────┘ │
│                                             │
│  index: Arc<RwLock<HNSWIndex>>             │
│  ┌───────────────────────────────────────┐ │
│  │ HNSW Graph Structure                  │ │
│  │  • Node connections (M=32)            │ │
│  │  • Hierarchical layers                │ │
│  │  • Vector storage                     │ │
│  └───────────────────────────────────────┘ │
│                                             │
│  embedding_cache: EmbeddingCache            │
│  ┌───────────────────────────────────────┐ │
│  │ Moka Cache (LRU + TTL)                │ │
│  │  • Max: 10k entries                   │ │
│  │  • TTL: 1 hour                        │ │
│  │  • ~15MB memory                       │ │
│  └───────────────────────────────────────┘ │
│                                             │
│  query_cache: QueryCache                    │
│  ┌───────────────────────────────────────┐ │
│  │ Moka Cache (LRU + TTL)                │ │
│  │  • Max: 1k entries                    │ │
│  │  • TTL: 5 minutes                     │ │
│  │  • ~100KB memory                      │ │
│  └───────────────────────────────────────┘ │
└─────────────────────────────────────────────┘

Disk Layout:
┌─────────────────────────────────────────────┐
│ ./data/semantic_index.bin                   │
├─────────────────────────────────────────────┤
│                                             │
│  Magic Header                               │
│  Version Info                               │
│                                             │
│  Index Metadata:                            │
│  ├─ Dimension: 384                          │
│  ├─ Total Vectors: N                        │
│  ├─ HNSW Parameters: M, ef_construction     │
│  └─ Similarity Metric: Cosine               │
│                                             │
│  Document Mappings:                         │
│  ├─ doc_map: HashMap<usize, DocumentId>     │
│  └─ reverse_map: HashMap<DocumentId, usize> │
│                                             │
│  Vector Storage:                            │
│  ├─ vectors: HashMap<usize, Vec<f32>>       │
│  └─ (Serialized with bincode)               │
│                                             │
│  Checksum                                   │
└─────────────────────────────────────────────┘
```

## Thread Safety Model

```
┌──────────────────────────────────────┐
│ Multiple Concurrent Search Threads   │
└────────┬─────────────────────────────┘
         │
         │ Read-only access
         │
         ▼
┌──────────────────────────────────────┐
│  Arc<RwLock<HNSWIndex>>              │
│                                      │
│  RwLock allows:                      │
│  • Multiple concurrent readers       │
│  • Exclusive writer access           │
│  • No data races                     │
└──────────────────────────────────────┘

┌──────────────────────────────────────┐
│ Concurrent Document Storage          │
│                                      │
│  DashMap<DocumentId, Document>       │
│                                      │
│  • Lock-free concurrent HashMap      │
│  • Safe for concurrent reads/writes  │
│  • No global lock                    │
└──────────────────────────────────────┘

┌──────────────────────────────────────┐
│ Cache Layer                          │
│                                      │
│  Moka Cache                          │
│                                      │
│  • Thread-safe by design             │
│  • Concurrent eviction               │
│  • No explicit locking needed        │
└──────────────────────────────────────┘
```

## Performance Characteristics

```
Operation Complexity:
┌─────────────────────┬──────────────┬────────────┐
│ Operation           │ Time         │ Space      │
├─────────────────────┼──────────────┼────────────┤
│ Insert              │ O(log N)     │ O(1)       │
│ Batch Insert (k)    │ O(k * log N) │ O(k)       │
│ Search (k results)  │ O(log N)     │ O(k)       │
│ Remove              │ O(1)         │ O(1)       │
│ Embedding Generate  │ O(text_len)  │ O(dim)     │
│ Query Processing    │ O(query_len) │ O(1)       │
│ Result Ranking      │ O(k * log k) │ O(k)       │
└─────────────────────┴──────────────┴────────────┘

Cache Hit Rates (Typical):
┌─────────────────┬──────────┬───────────┐
│ Cache Type      │ Hit Rate │ Speedup   │
├─────────────────┼──────────┼───────────┤
│ Embedding Cache │ 40-60%   │ 1,500x    │
│ Query Cache     │ 20-40%   │ 40x       │
└─────────────────┴──────────┴───────────┘

Memory Usage:
┌─────────────────┬────────────────────────┐
│ Component       │ Memory/Document        │
├─────────────────┼────────────────────────┤
│ Embedding       │ 1.5 KB (384 * 4 bytes) │
│ Metadata        │ ~200 bytes             │
│ HNSW Overhead   │ ~400 bytes             │
│ Total           │ ~2.1 KB                │
└─────────────────┴────────────────────────┘

For 100k documents:
  • Index: ~210 MB
  • Embedding Cache: ~15 MB
  • Query Cache: ~100 KB
  • Total: ~225 MB
```

## Error Handling Flow

```
User Operation
    │
    ▼
Try Primary Operation
    │
    ├─ Success → Return result
    │
    └─ Error
        │
        ├─ Provider Error
        │   │
        │   ├─ Try Fallback Provider #1
        │   │   ├─ Success → Return result
        │   │   └─ Error → Try next fallback
        │   │
        │   └─ Try Fallback Provider #2
        │       ├─ Success → Return result
        │       └─ Error → Return error to user
        │
        ├─ Dimension Mismatch
        │   └─ Return descriptive error
        │
        ├─ IO Error
        │   └─ Log & return error
        │
        └─ Other Errors
            └─ Log & return error

All errors are:
  • Logged with tracing
  • Wrapped in SemanticError enum
  • Include context
  • Gracefully handled
```

## Monitoring & Observability

```
┌─────────────────────────────────────────┐
│ Application Metrics (Prometheus)        │
├─────────────────────────────────────────┤
│                                         │
│  Counters:                              │
│  • searches_total                       │
│  • embeddings_generated_total           │
│  • cache_hits_total                     │
│  • cache_misses_total                   │
│  • errors_total (by type)               │
│                                         │
│  Histograms:                            │
│  • search_duration_seconds              │
│  • embedding_generation_duration        │
│  • index_operation_duration             │
│                                         │
│  Gauges:                                │
│  • index_size_documents                 │
│  • cache_size_entries                   │
│  • active_searches                      │
│                                         │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│ Grafana Dashboard                       │
├─────────────────────────────────────────┤
│                                         │
│  Panels:                                │
│  • Query Latency (p50, p95, p99)        │
│  • Throughput (queries/second)          │
│  • Cache Hit Rate (%)                   │
│  • Index Growth Over Time               │
│  • Error Rate                           │
│  • Provider Health                      │
│                                         │
└─────────────────────────────────────────┘
```

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Load Balancer                       │
└────────────────┬────────────────────┬───────────────────┘
                 │                    │
     ┌───────────┴───────┐   ┌────────┴──────────┐
     ▼                   ▼   ▼                   ▼
┌──────────┐      ┌──────────┐      ┌──────────┐
│ Instance │      │ Instance │      │ Instance │
│    #1    │      │    #2    │      │    #3    │
├──────────┤      ├──────────┤      ├──────────┤
│ Engine   │      │ Engine   │      │ Engine   │
│ Cache    │      │ Cache    │      │ Cache    │
└──────────┘      └──────────┘      └──────────┘
     │                   │                   │
     └───────────────────┴───────────────────┘
                         │
                         ▼
            ┌────────────────────────┐
            │  Shared Index Storage  │
            │  (NFS/S3/etc)          │
            └────────────────────────┘
                         │
                         ▼
            ┌────────────────────────┐
            │  Metrics & Logging     │
            │  (Prometheus/Grafana)  │
            └────────────────────────┘

Features:
• Horizontal scaling
• Load balancing
• Health checks
• Graceful shutdown
• Rolling updates
```

---

**Document Version:** 1.0.0
**Last Updated:** 2025-10-20
**Status:** Complete
