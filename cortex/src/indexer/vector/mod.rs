pub mod hnsw;

use anyhow::Result;

pub use hnsw::{HnswConfig, HnswIndex};

/// Vector dimension for AllMiniLML6V2 model
pub const VECTOR_DIM: usize = 384;

/// Vector indexing trait
pub trait VectorIndex: Send + Sync {
    /// Add a vector to the index
    fn add_vector(&mut self, id: &str, vector: &[f32]) -> Result<()>;

    /// Search for k nearest neighbors
    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>>;

    /// Remove a vector from the index
    fn remove_vector(&mut self, id: &str) -> Result<()>;

    /// Get the number of vectors in the index
    fn len(&self) -> usize;

    /// Check if the index is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Save the index to disk
    fn save(&self, path: &std::path::Path) -> Result<()>;

    /// Load the index from disk
    fn load(path: &std::path::Path) -> Result<Self>
    where
        Self: Sized;
}
