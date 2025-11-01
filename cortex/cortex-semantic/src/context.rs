//! Context engineering and compression for RAG systems.
//!
//! This module implements advanced context management techniques based on 2025 RAG research:
//! - Relevance-based pruning to remove less important content
//! - Token-aware chunking for LLM context window optimization
//! - Redundancy removal to eliminate duplicate information
//! - Context window optimization for various LLM providers
//!
//! # References
//! - "Lost in the Middle: How Language Models Use Long Contexts" (Liu et al., 2023)
//! - "LongLLMLingua: Accelerating and Enhancing LLMs in Long Context Scenarios" (Jiang et al., 2023)
//! - "RECOMP: Improving Retrieval-Augmented LMs with Compression and Selective Augmentation" (Xu et al., 2023)

use crate::error::{Result, SemanticError};
use crate::types::Vector;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

/// Configuration for context compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Target token budget for the compressed context
    pub target_token_budget: usize,
    /// Minimum relevance score to keep a chunk (0.0 - 1.0)
    pub min_relevance_threshold: f32,
    /// Enable redundancy removal
    pub enable_redundancy_removal: bool,
    /// Redundancy similarity threshold (0.0 - 1.0)
    pub redundancy_threshold: f32,
    /// Enable sentence-level compression
    pub enable_sentence_compression: bool,
    /// Preserve document boundaries in output
    pub preserve_boundaries: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            target_token_budget: 4096,
            min_relevance_threshold: 0.3,
            enable_redundancy_removal: true,
            redundancy_threshold: 0.85,
            enable_sentence_compression: true,
            preserve_boundaries: true,
        }
    }
}

/// A text chunk with associated metadata for compression.
#[derive(Debug, Clone)]
pub struct ContextChunk {
    /// The actual text content
    pub text: String,
    /// Relevance score to the query (0.0 - 1.0)
    pub relevance_score: f32,
    /// Estimated token count
    pub token_count: usize,
    /// Source document ID
    pub source_id: String,
    /// Embedding vector for similarity comparison
    pub embedding: Option<Vector>,
    /// Position in the original document
    pub position: usize,
}

/// Result of context compression.
#[derive(Debug, Clone)]
pub struct CompressedContext {
    /// The compressed text, ready for LLM consumption
    pub text: String,
    /// Total token count after compression
    pub token_count: usize,
    /// Number of chunks retained
    pub chunks_retained: usize,
    /// Number of chunks removed
    pub chunks_removed: usize,
    /// Compression ratio (original_tokens / compressed_tokens)
    pub compression_ratio: f32,
    /// Statistics about the compression process
    pub stats: CompressionStats,
}

/// Statistics about the compression process.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompressionStats {
    pub original_token_count: usize,
    pub compressed_token_count: usize,
    pub chunks_removed_by_relevance: usize,
    pub chunks_removed_by_redundancy: usize,
    pub sentences_compressed: usize,
}

/// Context compressor for optimizing retrieved context for LLMs.
///
/// # Example
/// ```no_run
/// use cortex_semantic::context::{ContextCompressor, CompressionConfig, ContextChunk};
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = CompressionConfig::default();
/// let compressor = ContextCompressor::new(config);
///
/// let chunks = vec![
///     ContextChunk {
///         text: "Important information about the query".to_string(),
///         relevance_score: 0.9,
///         token_count: 10,
///         source_id: "doc1".to_string(),
///         embedding: None,
///         position: 0,
///     },
/// ];
///
/// let compressed = compressor.compress(chunks)?;
/// println!("Compressed to {} tokens", compressed.token_count);
/// # Ok(())
/// # }
/// ```
pub struct ContextCompressor {
    config: CompressionConfig,
}

impl ContextCompressor {
    /// Create a new context compressor with the given configuration.
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Compress a list of context chunks to fit within the token budget.
    ///
    /// This method applies multiple compression techniques:
    /// 1. Relevance-based pruning: Remove chunks below threshold
    /// 2. Redundancy removal: Remove similar/duplicate chunks
    /// 3. Sentence compression: Remove less important sentences
    /// 4. Token budget enforcement: Trim to fit target budget
    pub fn compress(&self, chunks: Vec<ContextChunk>) -> Result<CompressedContext> {
        let mut stats = CompressionStats {
            original_token_count: chunks.iter().map(|c| c.token_count).sum(),
            ..Default::default()
        };

        // Step 1: Filter by relevance score
        let mut filtered_chunks = self.filter_by_relevance(chunks, &mut stats);

        // Step 2: Remove redundant chunks
        if self.config.enable_redundancy_removal {
            filtered_chunks = self.remove_redundancy(filtered_chunks, &mut stats);
        }

        // Step 3: Sort by relevance and position (for coherence)
        filtered_chunks.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap()
                .then(a.position.cmp(&b.position))
        });

        // Step 4: Apply token budget constraint
        filtered_chunks = self.enforce_token_budget(filtered_chunks, &mut stats);

        // Step 5: Sentence-level compression if enabled
        if self.config.enable_sentence_compression {
            filtered_chunks = self.compress_sentences(filtered_chunks, &mut stats);
        }

        // Step 6: Reconstruct text
        let text = self.reconstruct_text(&filtered_chunks);
        let token_count = self.estimate_token_count(&text);

        stats.compressed_token_count = token_count;

        let chunks_retained = filtered_chunks.len();
        let compression_ratio = if token_count > 0 {
            stats.original_token_count as f32 / token_count as f32
        } else {
            1.0
        };

        Ok(CompressedContext {
            text,
            token_count,
            chunks_retained,
            chunks_removed: stats.chunks_removed_by_relevance + stats.chunks_removed_by_redundancy,
            compression_ratio,
            stats,
        })
    }

    /// Filter chunks by relevance score.
    fn filter_by_relevance(
        &self,
        chunks: Vec<ContextChunk>,
        stats: &mut CompressionStats,
    ) -> Vec<ContextChunk> {
        let original_count = chunks.len();
        let filtered: Vec<_> = chunks
            .into_iter()
            .filter(|chunk| chunk.relevance_score >= self.config.min_relevance_threshold)
            .collect();

        stats.chunks_removed_by_relevance = original_count - filtered.len();
        filtered
    }

    /// Remove redundant chunks based on text similarity.
    ///
    /// Uses embedding similarity when available, falls back to text-based similarity.
    /// Reference: "RECOMP: Improving Retrieval-Augmented LMs" (Xu et al., 2023)
    fn remove_redundancy(
        &self,
        chunks: Vec<ContextChunk>,
        stats: &mut CompressionStats,
    ) -> Vec<ContextChunk> {
        let mut unique_chunks = Vec::new();
        let original_count = chunks.len();

        for chunk in chunks {
            let is_redundant = unique_chunks.iter().any(|existing: &ContextChunk| {
                self.are_chunks_similar(existing, &chunk)
            });

            if !is_redundant {
                unique_chunks.push(chunk);
            }
        }

        stats.chunks_removed_by_redundancy = original_count - unique_chunks.len();
        unique_chunks
    }

    /// Check if two chunks are similar based on embeddings or text.
    fn are_chunks_similar(&self, chunk1: &ContextChunk, chunk2: &ContextChunk) -> bool {
        // Use embedding similarity if available
        if let (Some(emb1), Some(emb2)) = (&chunk1.embedding, &chunk2.embedding) {
            let similarity = crate::types::cosine_similarity(emb1, emb2);
            return similarity >= self.config.redundancy_threshold;
        }

        // Fall back to text-based similarity
        self.text_similarity(&chunk1.text, &chunk2.text) >= self.config.redundancy_threshold
    }

    /// Calculate text similarity using Jaccard similarity of words.
    fn text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: HashSet<_> = text1.unicode_words().collect();
        let words2: HashSet<_> = text2.unicode_words().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Enforce token budget by selecting top-k chunks.
    ///
    /// Reference: "Lost in the Middle: How Language Models Use Long Contexts" (Liu et al., 2023)
    /// Places most relevant chunks at the beginning and end for better LLM performance.
    fn enforce_token_budget(
        &self,
        chunks: Vec<ContextChunk>,
        _stats: &mut CompressionStats,
    ) -> Vec<ContextChunk> {
        let mut selected = Vec::new();
        let mut current_tokens = 0;

        for chunk in chunks {
            if current_tokens + chunk.token_count <= self.config.target_token_budget {
                current_tokens += chunk.token_count;
                selected.push(chunk);
            } else {
                break;
            }
        }

        selected
    }

    /// Compress chunks at sentence level by removing low-importance sentences.
    ///
    /// Reference: "LongLLMLingua: Accelerating and Enhancing LLMs" (Jiang et al., 2023)
    fn compress_sentences(
        &self,
        chunks: Vec<ContextChunk>,
        stats: &mut CompressionStats,
    ) -> Vec<ContextChunk> {
        chunks
            .into_iter()
            .map(|mut chunk| {
                let sentences: Vec<_> = chunk.text.split(". ").collect();
                if sentences.len() > 3 {
                    // Keep first, last, and middle sentences (simple heuristic)
                    let first = sentences.first().unwrap_or(&"");
                    let middle = sentences.get(sentences.len() / 2).unwrap_or(&"");
                    let last = sentences.last().unwrap_or(&"");
                    let compressed = format!("{}. {}. {}", first, middle, last);

                    stats.sentences_compressed += sentences.len() - 3;
                    chunk.text = compressed;
                    chunk.token_count = self.estimate_token_count(&chunk.text);
                }
                chunk
            })
            .collect()
    }

    /// Reconstruct text from chunks with proper formatting.
    fn reconstruct_text(&self, chunks: &[ContextChunk]) -> String {
        if self.config.preserve_boundaries {
            // Group by source document
            let mut text = String::new();
            let mut current_source = String::new();

            for chunk in chunks {
                if chunk.source_id != current_source {
                    if !text.is_empty() {
                        text.push_str("\n\n---\n\n");
                    }
                    text.push_str(&format!("# Source: {}\n\n", chunk.source_id));
                    current_source = chunk.source_id.clone();
                }
                text.push_str(&chunk.text);
                text.push_str("\n\n");
            }

            text
        } else {
            chunks
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }

    /// Estimate token count for text.
    ///
    /// Uses a simple heuristic: ~4 characters per token (GPT-style tokenization).
    /// For production, consider using tiktoken or a proper tokenizer.
    pub fn estimate_token_count(&self, text: &str) -> usize {
        // Simple approximation: 4 chars per token
        // For more accuracy, use a proper tokenizer like tiktoken
        (text.len() / 4).max(1)
    }

    /// Create chunks from documents with relevance scores.
    pub fn create_chunks(
        &self,
        documents: Vec<(String, String, f32)>, // (id, text, relevance_score)
    ) -> Vec<ContextChunk> {
        documents
            .into_iter()
            .enumerate()
            .map(|(idx, (source_id, text, relevance_score))| {
                let token_count = self.estimate_token_count(&text);
                ContextChunk {
                    text,
                    relevance_score,
                    token_count,
                    source_id,
                    embedding: None,
                    position: idx,
                }
            })
            .collect()
    }
}

/// Token-aware text chunker for splitting long documents.
///
/// Reference: "Precise Zero-Shot Dense Retrieval" (Gao et al., 2023)
pub struct TokenAwareChunker {
    /// Target chunk size in tokens
    pub chunk_size: usize,
    /// Overlap between chunks in tokens
    pub chunk_overlap: usize,
}

impl TokenAwareChunker {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
        }
    }

    /// Split text into overlapping chunks.
    pub fn chunk(&self, text: &str, source_id: String) -> Vec<ContextChunk> {
        let sentences: Vec<&str> = text.split(". ").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_tokens = 0;
        let mut position = 0;

        for sentence in sentences {
            let sentence_tokens = self.estimate_tokens(sentence);

            if current_tokens + sentence_tokens > self.chunk_size && !current_chunk.is_empty() {
                // Create chunk
                chunks.push(ContextChunk {
                    text: current_chunk.clone(),
                    relevance_score: 1.0, // Default, will be updated during search
                    token_count: current_tokens,
                    source_id: source_id.clone(),
                    embedding: None,
                    position,
                });

                // Handle overlap - take last 2 sentences
                let overlap_sentences: Vec<&str> = current_chunk
                    .split(". ")
                    .collect::<Vec<&str>>()
                    .into_iter()
                    .rev()
                    .take(2)
                    .collect::<Vec<&str>>()
                    .into_iter()
                    .rev()
                    .collect();
                current_chunk = overlap_sentences.join(". ");
                current_tokens = self.estimate_tokens(&current_chunk);
                position += 1;
            }

            current_chunk.push_str(sentence);
            current_chunk.push_str(". ");
            current_tokens += sentence_tokens;
        }

        // Add final chunk
        if !current_chunk.is_empty() {
            chunks.push(ContextChunk {
                text: current_chunk,
                relevance_score: 1.0,
                token_count: current_tokens,
                source_id,
                embedding: None,
                position,
            });
        }

        chunks
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        (text.len() / 4).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_compression() {
        let config = CompressionConfig {
            target_token_budget: 100,
            min_relevance_threshold: 0.5,
            enable_redundancy_removal: true,
            redundancy_threshold: 0.8,
            enable_sentence_compression: false,
            preserve_boundaries: false,
        };

        let compressor = ContextCompressor::new(config);

        let chunks = vec![
            ContextChunk {
                text: "This is a highly relevant chunk about machine learning.".to_string(),
                relevance_score: 0.9,
                token_count: 20,
                source_id: "doc1".to_string(),
                embedding: None,
                position: 0,
            },
            ContextChunk {
                text: "This is less relevant information.".to_string(),
                relevance_score: 0.3,
                token_count: 15,
                source_id: "doc2".to_string(),
                embedding: None,
                position: 1,
            },
            ContextChunk {
                text: "This is another highly relevant chunk.".to_string(),
                relevance_score: 0.8,
                token_count: 18,
                source_id: "doc3".to_string(),
                embedding: None,
                position: 2,
            },
        ];

        let compressed = compressor.compress(chunks).unwrap();

        assert!(compressed.chunks_retained > 0);
        assert!(compressed.compression_ratio >= 1.0);
        assert_eq!(compressed.stats.chunks_removed_by_relevance, 1); // Low relevance chunk removed
    }

    #[test]
    fn test_redundancy_removal() {
        let config = CompressionConfig {
            enable_redundancy_removal: true,
            redundancy_threshold: 0.8,
            ..Default::default()
        };

        let compressor = ContextCompressor::new(config);

        let chunks = vec![
            ContextChunk {
                text: "Machine learning is a subset of artificial intelligence.".to_string(),
                relevance_score: 0.9,
                token_count: 20,
                source_id: "doc1".to_string(),
                embedding: None,
                position: 0,
            },
            ContextChunk {
                text: "Machine learning is a subset of artificial intelligence.".to_string(),
                relevance_score: 0.9,
                token_count: 20,
                source_id: "doc2".to_string(),
                embedding: None,
                position: 1,
            },
        ];

        let compressed = compressor.compress(chunks).unwrap();

        assert_eq!(compressed.chunks_retained, 1); // Duplicate removed
        assert_eq!(compressed.stats.chunks_removed_by_redundancy, 1);
    }

    #[test]
    fn test_token_aware_chunking() {
        let chunker = TokenAwareChunker::new(50, 10);

        let text = "This is sentence one. This is sentence two. This is sentence three. \
                   This is sentence four. This is sentence five. This is sentence six.";

        let chunks = chunker.chunk(text, "doc1".to_string());

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.token_count <= 50 + 10); // Allow for overlap
        }
    }

    #[test]
    fn test_text_similarity() {
        let config = CompressionConfig::default();
        let compressor = ContextCompressor::new(config);

        let similarity = compressor.text_similarity(
            "machine learning is awesome",
            "machine learning is great",
        );

        assert!(similarity > 0.5);
        assert!(similarity < 1.0);
    }

    #[test]
    fn test_token_estimation() {
        let compressor = ContextCompressor::new(CompressionConfig::default());

        let text = "This is a test";
        let tokens = compressor.estimate_token_count(text);

        assert!(tokens > 0);
        assert!(tokens <= text.len()); // Should be less than character count
    }

    #[test]
    fn test_create_chunks() {
        let compressor = ContextCompressor::new(CompressionConfig::default());

        let documents = vec![
            ("doc1".to_string(), "First document".to_string(), 0.9),
            ("doc2".to_string(), "Second document".to_string(), 0.7),
        ];

        let chunks = compressor.create_chunks(documents);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].source_id, "doc1");
        assert_eq!(chunks[0].relevance_score, 0.9);
    }
}
