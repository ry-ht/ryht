//! Virtual filesystem implementation.

use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_core::traits::{Storage, VirtualFilesystem, FileMetadata};
use cortex_core::types::Document;
use async_trait::async_trait;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Virtual filesystem implementation
pub struct Vfs {
    storage: Arc<dyn Storage>,
    cache: Arc<DashMap<String, Vec<u8>>>,
    base_path: PathBuf,
}

impl Vfs {
    /// Create a new virtual filesystem
    pub fn new(storage: Arc<dyn Storage>, base_path: PathBuf) -> Self {
        Self {
            storage,
            cache: Arc::new(DashMap::new()),
            base_path,
        }
    }

    /// Resolve a virtual path to a physical path
    fn resolve_path(&self, path: &str) -> PathBuf {
        self.base_path.join(path.trim_start_matches('/'))
    }

    /// Calculate content hash
    fn hash_content(content: &[u8]) -> String {
        let hash = blake3::hash(content);
        hash.to_hex().to_string()
    }

    /// Load document content from disk
    async fn load_from_disk(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path)
            .await
            .map_err(|e| CortexError::vfs(format!("Failed to read file: {}", e)))
    }

    /// Write content to disk
    async fn write_to_disk(&self, path: &Path, content: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| CortexError::vfs(format!("Failed to create directory: {}", e)))?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| CortexError::vfs(format!("Failed to write file: {}", e)))
    }
}

#[async_trait]
impl VirtualFilesystem for Vfs {
    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(cached) = self.cache.get(path) {
            tracing::debug!("VFS cache hit for: {}", path);
            return Ok(cached.clone());
        }

        // Load from disk
        let physical_path = self.resolve_path(path);
        let content = self.load_from_disk(&physical_path).await?;

        // Cache the content
        self.cache.insert(path.to_string(), content.clone());

        Ok(content)
    }

    async fn write(&self, path: &str, content: &[u8]) -> Result<()> {
        let physical_path = self.resolve_path(path);
        self.write_to_disk(&physical_path, content).await?;

        // Update cache
        self.cache.insert(path.to_string(), content.to_vec());

        Ok(())
    }

    async fn list(&self, path: &str) -> Result<Vec<String>> {
        let physical_path = self.resolve_path(path);

        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(physical_path)
            .await
            .map_err(|e| CortexError::vfs(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = read_dir.next_entry()
            .await
            .map_err(|e| CortexError::vfs(format!("Failed to read directory entry: {}", e)))?
        {
            if let Ok(name) = entry.file_name().into_string() {
                entries.push(name);
            }
        }

        Ok(entries)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let physical_path = self.resolve_path(path);
        Ok(physical_path.exists())
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let physical_path = self.resolve_path(path);

        let metadata = tokio::fs::metadata(&physical_path)
            .await
            .map_err(|e| CortexError::vfs(format!("Failed to get metadata: {}", e)))?;

        let created_at = metadata.created()
            .map(|t| chrono::DateTime::from(t))
            .unwrap_or_else(|_| chrono::Utc::now());

        let modified_at = metadata.modified()
            .map(|t| chrono::DateTime::from(t))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(FileMetadata {
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            created_at,
            modified_at,
        })
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let physical_path = self.resolve_path(path);

        if physical_path.is_dir() {
            tokio::fs::remove_dir_all(&physical_path)
                .await
                .map_err(|e| CortexError::vfs(format!("Failed to delete directory: {}", e)))?;
        } else {
            tokio::fs::remove_file(&physical_path)
                .await
                .map_err(|e| CortexError::vfs(format!("Failed to delete file: {}", e)))?;
        }

        // Remove from cache
        self.cache.remove(path);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let content = b"hello world";
        let hash = Vfs::hash_content(content);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // Blake3 produces 256-bit hash (64 hex chars)
    }
}
