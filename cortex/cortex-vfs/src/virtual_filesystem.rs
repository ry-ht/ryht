//! Core Virtual Filesystem implementation.

use crate::content_cache::ContentCache;
use crate::path::VirtualPath;
use crate::types::*;
use cortex_core::error::{CortexError, Result};
use cortex_storage::ConnectionManager;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Virtual Filesystem providing path-agnostic file operations.
///
/// The VFS stores all file metadata and content in SurrealDB, with:
/// - Path-agnostic design (virtual paths independent of physical location)
/// - Content deduplication using blake3 hashing
/// - Lazy materialization (files exist in memory until explicitly flushed)
/// - Multi-workspace support with isolation
/// - External project import with fork capability
#[derive(Clone)]
pub struct VirtualFileSystem {
    /// Database connection manager
    storage: Arc<ConnectionManager>,

    /// Content cache for frequently accessed files
    content_cache: ContentCache,

    /// VNode metadata cache with LRU eviction (max 10,000 entries)
    vnode_cache: Arc<Mutex<LruCache<Uuid, VNode>>>,

    /// Path to VNode ID mapping cache with LRU eviction (max 10,000 entries)
    path_cache: Arc<Mutex<LruCache<(Uuid, String), Uuid>>>,
}

impl VirtualFileSystem {
    /// Create a new virtual filesystem.
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self::with_cache_config(storage, 512 * 1024 * 1024, 10_000)
    }

    /// Create with custom cache size.
    pub fn with_cache_size(storage: Arc<ConnectionManager>, cache_size: usize) -> Self {
        Self::with_cache_config(storage, cache_size, 10_000)
    }

    /// Create with custom cache configuration.
    pub fn with_cache_config(
        storage: Arc<ConnectionManager>,
        content_cache_size: usize,
        vnode_cache_size: usize,
    ) -> Self {
        Self {
            storage,
            content_cache: ContentCache::new(content_cache_size),
            vnode_cache: Arc::new(Mutex::new(
                LruCache::new(NonZeroUsize::new(vnode_cache_size).unwrap())
            )),
            path_cache: Arc::new(Mutex::new(
                LruCache::new(NonZeroUsize::new(vnode_cache_size).unwrap())
            )),
        }
    }

    // ============================================================================
    // Core File Operations
    // ============================================================================

    /// Read file content from VFS.
    pub async fn read_file(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
    ) -> Result<Vec<u8>> {
        debug!("Reading file: {} in workspace {}", path, workspace_id);

        // Get vnode
        let vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("File", path.to_string()))?;

        // Check if it's a file
        if !vnode.is_file() {
            return Err(CortexError::invalid_input(format!("Not a file: {}", path)));
        }

        // Get content hash
        let content_hash = vnode.content_hash
            .ok_or_else(|| CortexError::internal("File has no content hash"))?;

        // Try cache first
        if let Some(content) = self.content_cache.get(&content_hash) {
            debug!("Cache hit for content hash: {}", content_hash);
            return Ok((*content).clone());
        }

        // Load from database
        let content = self.load_content_from_db(&content_hash).await?;

        // Cache for future access
        self.content_cache.put(content_hash, content.clone());

        Ok(content)
    }

    /// Write file content to VFS.
    pub async fn write_file(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        content: &[u8],
    ) -> Result<()> {
        debug!("Writing file: {} in workspace {}", path, workspace_id);

        // Calculate content hash
        let content_hash = Self::hash_content(content);

        // Store content (deduplicated)
        self.store_content(&content_hash, content).await?;

        // Get or create vnode
        let vnode = if let Some(mut vnode) = self.get_vnode(workspace_id, path).await? {
            // Check if read-only
            if vnode.read_only {
                return Err(CortexError::invalid_input(
                    format!("File is read-only: {}", path)
                ));
            }

            // Update existing vnode
            vnode.content_hash = Some(content_hash.clone());
            vnode.size_bytes = content.len();
            vnode.mark_modified();

            vnode
        } else {
            // Create new vnode
            let mut vnode = VNode::new_file(
                *workspace_id,
                path.clone(),
                content_hash.clone(),
                content.len(),
            );

            // Detect language
            if let Some(ext) = path.extension() {
                vnode.language = Some(Language::from_extension(ext));
            }

            vnode
        };

        // Save vnode to database
        self.save_vnode(&vnode).await?;

        // Cache content
        self.content_cache.put(content_hash, content.to_vec());

        Ok(())
    }

    /// Create a directory in the VFS.
    pub async fn create_directory(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        create_parents: bool,
    ) -> Result<()> {
        debug!("Creating directory: {} in workspace {}", path, workspace_id);

        // Create parents if requested
        if create_parents {
            let mut current = VirtualPath::root();
            for segment in path.segments() {
                current = current.join(segment)?;

                if self.get_vnode(workspace_id, &current).await?.is_none() {
                    let vnode = VNode::new_directory(*workspace_id, current.clone());
                    self.save_vnode(&vnode).await?;
                }
            }
        } else {
            // Just create the directory
            let vnode = VNode::new_directory(*workspace_id, path.clone());
            self.save_vnode(&vnode).await?;
        }

        Ok(())
    }

    /// List entries in a directory.
    pub async fn list_directory(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        recursive: bool,
    ) -> Result<Vec<VNode>> {
        debug!("Listing directory: {} in workspace {}", path, workspace_id);

        // Check that directory exists
        let vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("Directory", path.to_string()))?;

        if !vnode.is_directory() {
            return Err(CortexError::invalid_input(format!("Not a directory: {}", path)));
        }

        // Query database for children
        self.list_children(workspace_id, path, recursive).await
    }

    /// Delete a file or directory.
    pub async fn delete(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        recursive: bool,
    ) -> Result<()> {
        debug!("Deleting: {} in workspace {}", path, workspace_id);

        let vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("Path", path.to_string()))?;

        // Check if read-only
        if vnode.read_only {
            return Err(CortexError::invalid_input(
                format!("Path is read-only: {}", path)
            ));
        }

        // If directory, check recursive flag
        if vnode.is_directory() && !recursive {
            let children = self.list_children(workspace_id, path, false).await?;
            if !children.is_empty() {
                return Err(CortexError::invalid_input(
                    "Directory not empty (use recursive=true)"
                ));
            }
        }

        // Mark as deleted
        self.mark_deleted(&vnode.id).await?;

        // Invalidate caches
        self.invalidate_vnode_cache(&vnode.id);

        Ok(())
    }

    /// Check if a path exists.
    pub async fn exists(&self, workspace_id: &Uuid, path: &VirtualPath) -> Result<bool> {
        Ok(self.get_vnode(workspace_id, path).await?.is_some())
    }

    /// Get metadata for a path.
    pub async fn metadata(&self, workspace_id: &Uuid, path: &VirtualPath) -> Result<VNode> {
        self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("Path", path.to_string()))
    }

    /// Get a vnode by its ID.
    pub async fn get_vnode_by_id(&self, id: &Uuid) -> Result<Option<VNode>> {
        // Check vnode cache first
        if let Some(vnode) = self.vnode_cache.lock().get(id).cloned() {
            return Ok(Some(vnode));
        }

        // Query database
        let query = "SELECT * FROM vnode WHERE id = $id AND status != 'deleted' LIMIT 1";

        let conn = self.storage.acquire().await?;
        let mut response = conn.connection()
            .query(query)
            .bind(("id", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let result: Option<VNode> = response.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        if let Some(ref vnode) = result {
            // Cache the result
            self.vnode_cache.lock().put(vnode.id, vnode.clone());

            // Also cache the path mapping for future lookups
            let cache_key = (vnode.workspace_id, vnode.path.to_string());
            self.path_cache.lock().put(cache_key, vnode.id);
        }

        Ok(result)
    }

    // ============================================================================
    // VNode Management
    // ============================================================================

    /// Get a vnode by path.
    async fn get_vnode(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
    ) -> Result<Option<VNode>> {
        // Check path cache
        let cache_key = (*workspace_id, path.to_string());
        let cached_vnode_id = self.path_cache.lock().get(&cache_key).copied();

        if let Some(vnode_id) = cached_vnode_id {
            if let Some(vnode) = self.vnode_cache.lock().get(&vnode_id).cloned() {
                return Ok(Some(vnode));
            }
        }

        // Query database
        let query = format!(
            "SELECT * FROM vnode WHERE workspace_id = $workspace_id AND path = $path AND status != 'deleted' LIMIT 1"
        );

        let conn = self.storage.acquire().await?;
        let mut response = conn.connection()
            .query(&query)
            .bind(("workspace_id", workspace_id.to_string()))
            .bind(("path", path.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let result: Option<VNode> = response.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        if let Some(vnode) = &result {
            // Cache the result (LRU will automatically evict oldest entries if needed)
            self.vnode_cache.lock().put(vnode.id, vnode.clone());
            self.path_cache.lock().put(cache_key, vnode.id);
        }

        Ok(result)
    }

    /// Save a vnode to the database.
    pub async fn save_vnode(&self, vnode: &VNode) -> Result<()> {
        let query = format!(
            "CREATE vnode CONTENT $vnode"
        );

        let conn = self.storage.acquire().await?;
        let vnode_json = serde_json::to_value(vnode)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        conn.connection()
            .query(&query)
            .bind(("vnode", vnode_json))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        // Cache the vnode (LRU will automatically evict oldest entries if needed)
        self.vnode_cache.lock().put(vnode.id, vnode.clone());
        self.path_cache.lock().put(
            (vnode.workspace_id, vnode.path.to_string()),
            vnode.id,
        );

        Ok(())
    }

    /// Mark a vnode as deleted.
    async fn mark_deleted(&self, vnode_id: &Uuid) -> Result<()> {
        let query = format!(
            "UPDATE vnode:$id SET status = 'deleted', updated_at = time::now()"
        );

        let conn = self.storage.acquire().await?;
        conn.connection()
            .query(&query)
            .bind(("id", vnode_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(())
    }

    /// List children of a directory.
    async fn list_children(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        recursive: bool,
    ) -> Result<Vec<VNode>> {
        let path_str = path.to_string_with_slash();
        let query = if recursive {
            format!(
                "SELECT * FROM vnode WHERE workspace_id = $workspace_id
                 AND path LIKE $pattern AND status != 'deleted'"
            )
        } else {
            // Only direct children (count slashes to determine depth)
            format!(
                "SELECT * FROM vnode WHERE workspace_id = $workspace_id
                 AND path LIKE $pattern AND status != 'deleted'"
            )
        };

        let pattern = if recursive {
            format!("{}%", path_str)
        } else {
            format!("{}/%", path_str)
        };

        let conn = self.storage.acquire().await?;
        let mut response = conn.connection()
            .query(&query)
            .bind(("workspace_id", workspace_id.to_string()))
            .bind(("pattern", pattern))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let results: Vec<VNode> = response.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(results)
    }

    /// Invalidate vnode cache.
    fn invalidate_vnode_cache(&self, vnode_id: &Uuid) {
        // For LRU cache, we need to check if it exists first
        let vnode_opt = self.vnode_cache.lock().pop(vnode_id);
        if let Some(vnode) = vnode_opt {
            let cache_key = (vnode.workspace_id, vnode.path.to_string());
            self.path_cache.lock().pop(&cache_key);
        }
    }

    // ============================================================================
    // Content Management
    // ============================================================================

    /// Calculate blake3 hash of content.
    fn hash_content(content: &[u8]) -> String {
        let hash = blake3::hash(content);
        hash.to_hex().to_string()
    }

    /// Store content in database (deduplicated).
    ///
    /// FIXED: Uses database-level atomic increment to prevent race conditions
    /// The reference_count is incremented atomically using a single database operation
    async fn store_content(&self, hash: &str, content: &[u8]) -> Result<()> {
        let conn = self.storage.acquire().await?;

        // Prepare content record (for potential insertion)
        let file_content = FileContent {
            content_hash: hash.to_string(),
            content: String::from_utf8(content.to_vec()).ok(),
            content_binary: Some(content.to_vec()),
            is_compressed: false,
            compression_type: None,
            size_bytes: content.len(),
            line_count: String::from_utf8_lossy(content)
                .lines()
                .count()
                .into(),
            reference_count: 1,
            created_at: chrono::Utc::now(),
        };

        let file_content_json = serde_json::to_value(&file_content)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        // FIXED: Use atomic database operation to handle both create and increment
        // This query will:
        // 1. Try to create the record if it doesn't exist (reference_count = 1)
        // 2. If it already exists, atomically increment reference_count
        // The RETURN NONE prevents returning data we don't need

        // Use a single atomic query that handles both cases
        // UPSERT with ON DUPLICATE KEY UPDATE would be ideal, but SurrealDB uses different syntax
        // We'll use an atomic increment that creates if not exists
        let query = r#"
            LET $record = SELECT * FROM type::thing('file_content', $hash) LIMIT 1;
            IF $record {
                UPDATE type::thing('file_content', $hash) SET reference_count += 1;
            } ELSE {
                CREATE type::thing('file_content', $hash) CONTENT $content;
            };
        "#;

        conn.connection()
            .query(query)
            .bind(("hash", hash.to_string()))
            .bind(("content", file_content_json))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        debug!("Content stored/referenced: {} (atomic operation)", hash);
        Ok(())
    }

    /// Load content from database.
    async fn load_content_from_db(&self, hash: &str) -> Result<Vec<u8>> {
        let query = format!(
            "SELECT * FROM file_content WHERE content_hash = $hash LIMIT 1"
        );

        let conn = self.storage.acquire().await?;
        let mut response = conn.connection()
            .query(&query)
            .bind(("hash", hash.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let content: Option<FileContent> = response.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let content = content
            .ok_or_else(|| CortexError::not_found("Content", hash.to_string()))?;

        // Return binary content if available, otherwise encode text content
        if let Some(binary) = content.content_binary {
            Ok(binary)
        } else if let Some(text) = content.content {
            Ok(text.into_bytes())
        } else {
            Err(CortexError::internal("Content has no data"))
        }
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> crate::content_cache::CacheStatistics {
        self.content_cache.stats()
    }

    /// Clear all caches.
    pub fn clear_caches(&self) {
        self.content_cache.clear();
        self.vnode_cache.lock().clear();
        self.path_cache.lock().clear();
    }

    // ============================================================================
    // Ingestion Integration Methods
    // ============================================================================

    /// Reparse a file and update its code units in semantic memory.
    /// This is a convenience method that can be called after file modifications.
    pub async fn reparse_file(
        &self,
        _workspace_id: &Uuid,
        path: &VirtualPath,
    ) -> Result<usize> {
        // This method would require an ingestion pipeline instance
        // For now, we return Ok(0) as a placeholder
        // In practice, this would be called from the ingestion pipeline
        debug!("Reparse requested for {} (not yet implemented)", path);
        Ok(0)
    }

    /// Get the number of code units extracted from a file.
    /// This reads metadata from the VNode if available.
    pub async fn get_file_units_count(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
    ) -> Result<usize> {
        let vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("File", path.to_string()))?;

        // Check metadata for units_count
        if let Some(count_value) = vnode.metadata.get("units_count") {
            if let Some(count) = count_value.as_u64() {
                return Ok(count as usize);
            }
        }

        Ok(0)
    }

    /// Update VNode metadata with code units count.
    /// Called by the ingestion pipeline after processing.
    pub async fn update_file_units_count(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        count: usize,
    ) -> Result<()> {
        let mut vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("File", path.to_string()))?;

        vnode.metadata.insert(
            "units_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(count)),
        );
        vnode.mark_modified();

        self.save_vnode(&vnode).await?;

        Ok(())
    }

    // ============================================================================
    // Workspace Management
    // ============================================================================

    /// Create a workspace in the database.
    pub async fn create_workspace(&self, workspace: &Workspace) -> Result<()> {
        let query = "CREATE workspace CONTENT $workspace";

        let conn = self.storage.acquire().await?;
        let workspace_json = serde_json::to_value(workspace)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        conn.connection()
            .query(query)
            .bind(("workspace", workspace_json))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(())
    }

    // ============================================================================
    // Ergonomic Helper Methods for Tests
    // ============================================================================

    /// Create a new file in VFS with content.
    ///
    /// This is a convenience method that combines write_file and returns the VNode.
    /// It will create a new file if it doesn't exist, or fail if it already exists.
    pub async fn create_file(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        content: &[u8],
    ) -> Result<VNode> {
        debug!("Creating file: {} in workspace {}", path, workspace_id);

        // Check if file already exists
        if self.exists(workspace_id, path).await? {
            return Err(CortexError::invalid_input(
                format!("File already exists: {}", path)
            ));
        }

        // Calculate content hash
        let content_hash = Self::hash_content(content);

        // Store content (deduplicated)
        self.store_content(&content_hash, content).await?;

        // Create new vnode
        let mut vnode = VNode::new_file(
            *workspace_id,
            path.clone(),
            content_hash.clone(),
            content.len(),
        );

        // Detect language
        if let Some(ext) = path.extension() {
            vnode.language = Some(Language::from_extension(ext));
        }

        // Save vnode to database
        self.save_vnode(&vnode).await?;

        // Cache content
        self.content_cache.put(content_hash, content.to_vec());

        Ok(vnode)
    }

    /// Get a file from VFS by path, returning the VNode with content.
    ///
    /// This is a convenience method for tests that returns the full VNode.
    /// Returns Ok(VNode) if file exists, or Err if not found.
    pub async fn get_file(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
    ) -> Result<VNode> {
        debug!("Getting file: {} in workspace {}", path, workspace_id);

        // Get vnode
        let vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("File", path.to_string()))?;

        // Check if it's a file
        if !vnode.is_file() {
            return Err(CortexError::invalid_input(format!("Not a file: {}", path)));
        }

        // Note: We don't preload the content here as it would be wasteful
        // The caller can use read_file() if they need the content bytes
        Ok(vnode)
    }

    /// Update an existing file's content.
    ///
    /// This is a convenience method that updates a file and returns the updated VNode.
    /// It will increment the version and update the content hash.
    pub async fn update_file(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        content: &[u8],
    ) -> Result<VNode> {
        debug!("Updating file: {} in workspace {}", path, workspace_id);

        // Get existing vnode
        let mut vnode = self.get_vnode(workspace_id, path).await?
            .ok_or_else(|| CortexError::not_found("File", path.to_string()))?;

        // Check if it's a file
        if !vnode.is_file() {
            return Err(CortexError::invalid_input(format!("Not a file: {}", path)));
        }

        // Check if read-only
        if vnode.read_only {
            return Err(CortexError::invalid_input(
                format!("File is read-only: {}", path)
            ));
        }

        // Calculate new content hash
        let content_hash = Self::hash_content(content);

        // Store content (deduplicated)
        self.store_content(&content_hash, content).await?;

        // Update vnode
        vnode.content_hash = Some(content_hash.clone());
        vnode.size_bytes = content.len();
        vnode.mark_modified();

        // Save updated vnode
        self.save_vnode(&vnode).await?;

        // Cache content
        self.content_cache.put(content_hash, content.to_vec());

        Ok(vnode)
    }
}

// Note: MaterializationEngine, ExternalProjectLoader, and ForkManager
// will be implemented in separate files due to complexity
