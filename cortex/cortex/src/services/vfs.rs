//! VFS service layer
//!
//! Provides unified virtual filesystem operations for both API and MCP modules.

use anyhow::Result;
use chrono::{DateTime, Utc};
use cortex_vfs::{NodeType, VirtualFileSystem, VirtualPath, VNode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// VFS service for managing virtual files and directories
#[derive(Clone)]
pub struct VfsService {
    vfs: Arc<VirtualFileSystem>,
}

impl VfsService {
    /// Create a new VFS service
    pub fn new(vfs: Arc<VirtualFileSystem>) -> Self {
        Self { vfs }
    }

    /// Read file content
    pub async fn read_file(&self, workspace_id: &Uuid, path: &str) -> Result<Vec<u8>> {
        debug!("Reading file: {} in workspace {}", path, workspace_id);

        let vpath = VirtualPath::new(path)?;
        let content = self.vfs.read_file(workspace_id, &vpath).await?;

        Ok(content)
    }

    /// Write file content
    pub async fn write_file(&self, workspace_id: &Uuid, path: &str, content: &[u8]) -> Result<FileDetails> {
        info!("Writing file: {} in workspace {}", path, workspace_id);

        let vpath = VirtualPath::new(path)?;

        // Create parent directories if needed
        if let Some(parent) = vpath.parent() {
            self.vfs.create_directory(workspace_id, &parent, true).await?;
        }

        // Write file
        self.vfs.write_file(workspace_id, &vpath, content).await?;

        // Get metadata
        let vnode = self.vfs.metadata(workspace_id, &vpath).await?;

        Ok(FileDetails::from_vnode(vnode))
    }

    /// List directory contents
    pub async fn list_directory(&self, workspace_id: &Uuid, path: &str, recursive: bool) -> Result<Vec<FileDetails>> {
        debug!("Listing directory: {} in workspace {} (recursive: {})", path, workspace_id, recursive);

        let vpath = VirtualPath::new(path)?;
        let vnodes = self.vfs.list_directory(workspace_id, &vpath, recursive).await?;

        Ok(vnodes.into_iter().map(FileDetails::from_vnode).collect())
    }

    /// Delete file or directory
    pub async fn delete(&self, workspace_id: &Uuid, path: &str, recursive: bool) -> Result<()> {
        info!("Deleting: {} in workspace {} (recursive: {})", path, workspace_id, recursive);

        let vpath = VirtualPath::new(path)?;
        self.vfs.delete(workspace_id, &vpath, recursive).await?;

        Ok(())
    }

    /// Create directory
    pub async fn create_directory(&self, workspace_id: &Uuid, path: &str, create_parents: bool) -> Result<FileDetails> {
        info!("Creating directory: {} in workspace {} (parents: {})", path, workspace_id, create_parents);

        let vpath = VirtualPath::new(path)?;
        self.vfs.create_directory(workspace_id, &vpath, create_parents).await?;

        // Get metadata
        let vnode = self.vfs.metadata(workspace_id, &vpath).await?;

        Ok(FileDetails::from_vnode(vnode))
    }

    /// Get file/directory metadata
    pub async fn get_metadata(&self, workspace_id: &Uuid, path: &str) -> Result<FileDetails> {
        debug!("Getting metadata: {} in workspace {}", path, workspace_id);

        let vpath = VirtualPath::new(path)?;
        let vnode = self.vfs.metadata(workspace_id, &vpath).await?;

        Ok(FileDetails::from_vnode(vnode))
    }

    /// Get file/directory metadata by ID
    pub async fn get_file_by_id(&self, id: &Uuid) -> Result<FileDetails> {
        debug!("Getting file by ID: {}", id);

        let vnode = self.vfs.get_vnode_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found with ID: {}", id))?;

        Ok(FileDetails::from_vnode(vnode))
    }

    /// Read file content by ID
    pub async fn read_file_by_id(&self, id: &Uuid) -> Result<Vec<u8>> {
        debug!("Reading file by ID: {}", id);

        let vnode = self.vfs.get_vnode_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found with ID: {}", id))?;

        // Ensure it's a file
        if vnode.node_type != NodeType::File && vnode.node_type != NodeType::Document {
            anyhow::bail!("ID {} is not a file", id);
        }

        // Read content using workspace_id and path
        let content = self.vfs.read_file(&vnode.workspace_id, &vnode.path).await?;

        Ok(content)
    }

    /// Update file content by ID
    pub async fn update_file_by_id(&self, id: &Uuid, content: &[u8]) -> Result<FileDetails> {
        info!("Updating file by ID: {}", id);

        let vnode = self.vfs.get_vnode_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found with ID: {}", id))?;

        // Ensure it's a file
        if vnode.node_type != NodeType::File && vnode.node_type != NodeType::Document {
            anyhow::bail!("ID {} is not a file", id);
        }

        // Update content using workspace_id and path
        self.vfs.write_file(&vnode.workspace_id, &vnode.path, content).await?;

        // Get updated metadata
        let updated_vnode = self.vfs.metadata(&vnode.workspace_id, &vnode.path).await?;

        Ok(FileDetails::from_vnode(updated_vnode))
    }

    /// Delete file/directory by ID
    pub async fn delete_by_id(&self, id: &Uuid, recursive: bool) -> Result<()> {
        info!("Deleting by ID: {} (recursive: {})", id, recursive);

        let vnode = self.vfs.get_vnode_by_id(id).await?
            .ok_or_else(|| anyhow::anyhow!("File not found with ID: {}", id))?;

        // Delete using workspace_id and path
        self.vfs.delete(&vnode.workspace_id, &vnode.path, recursive).await?;

        Ok(())
    }

    /// Check if path exists
    pub async fn exists(&self, workspace_id: &Uuid, path: &str) -> Result<bool> {
        let vpath = VirtualPath::new(path)?;
        let exists = self.vfs.exists(workspace_id, &vpath).await?;

        Ok(exists)
    }

    /// Move/rename file or directory
    pub async fn move_node(&self, workspace_id: &Uuid, source_path: &str, target_path: &str) -> Result<FileDetails> {
        info!("Moving: {} -> {} in workspace {}", source_path, target_path, workspace_id);

        let source = VirtualPath::new(source_path)?;
        let target = VirtualPath::new(target_path)?;

        // Read source content
        let content = self.vfs.read_file(workspace_id, &source).await?;

        // Write to target
        self.vfs.write_file(workspace_id, &target, &content).await?;

        // Delete source
        self.vfs.delete(workspace_id, &source, false).await?;

        // Get target metadata
        let vnode = self.vfs.metadata(workspace_id, &target).await?;

        Ok(FileDetails::from_vnode(vnode))
    }

    /// Copy file or directory
    pub async fn copy_node(&self, workspace_id: &Uuid, source_path: &str, target_path: &str, recursive: bool) -> Result<FileDetails> {
        info!("Copying: {} -> {} in workspace {} (recursive: {})", source_path, target_path, workspace_id, recursive);

        let source = VirtualPath::new(source_path)?;
        let target = VirtualPath::new(target_path)?;

        // Get source metadata
        let source_node = self.vfs.metadata(workspace_id, &source).await?;

        if source_node.is_file() {
            // Copy file
            let content = self.vfs.read_file(workspace_id, &source).await?;
            self.vfs.write_file(workspace_id, &target, &content).await?;
        } else if source_node.is_directory() {
            if !recursive {
                anyhow::bail!("Cannot copy directory without recursive=true");
            }

            // Create target directory
            self.vfs.create_directory(workspace_id, &target, true).await?;

            // Recursively copy contents
            let children = self.vfs.list_directory(workspace_id, &source, true).await?;

            for child in children {
                if child.is_file() {
                    // Get relative path from source
                    let child_path_str = child.path.to_string();
                    let source_path_str = source.to_string();
                    let rel_path = child_path_str
                        .strip_prefix(&source_path_str)
                        .unwrap_or("")
                        .trim_start_matches('/');

                    if !rel_path.is_empty() {
                        let child_target = target.join(rel_path)?;

                        // Copy file
                        let content = self.vfs.read_file(workspace_id, &child.path).await?;
                        self.vfs.write_file(workspace_id, &child_target, &content).await?;
                    }
                }
            }
        }

        // Get target metadata
        let vnode = self.vfs.metadata(workspace_id, &target).await?;

        Ok(FileDetails::from_vnode(vnode))
    }

    /// Build directory tree
    pub async fn get_tree(&self, workspace_id: &Uuid, path: &str, max_depth: usize) -> Result<DirectoryTree> {
        debug!("Getting tree: {} in workspace {} (max_depth: {})", path, workspace_id, max_depth);

        let vpath = VirtualPath::new(path)?;
        let root_node = self.vfs.metadata(workspace_id, &vpath).await?;

        let tree = self.build_tree_recursive(workspace_id, &root_node, 0, max_depth).await?;

        Ok(tree)
    }

    // Helper to build tree recursively
    fn build_tree_recursive<'a>(
        &'a self,
        workspace_id: &'a Uuid,
        vnode: &'a VNode,
        current_depth: usize,
        max_depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<DirectoryTree>> + Send + 'a>> {
        Box::pin(async move {
            let mut tree = DirectoryTree {
                name: vnode.path.file_name().unwrap_or("/").to_string(),
                path: vnode.path.to_string(),
                node_type: format!("{:?}", vnode.node_type).to_lowercase(),
                size_bytes: if vnode.is_file() { Some(vnode.size_bytes as u64) } else { None },
                children: None,
            };

            // If it's a directory and we haven't reached max depth, get children
            if vnode.is_directory() && current_depth < max_depth {
                let children_nodes = self.vfs.list_directory(workspace_id, &vnode.path, false).await?;

                let mut children = Vec::new();
                for child in children_nodes {
                    let child_tree = self.build_tree_recursive(workspace_id, &child, current_depth + 1, max_depth).await?;
                    children.push(child_tree);
                }

                if !children.is_empty() {
                    tree.children = Some(children);
                }
            }

            Ok(tree)
        })
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDetails {
    pub id: String,
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub size_bytes: u64,
    pub language: Option<String>,
    pub permissions: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

impl FileDetails {
    fn from_vnode(vnode: VNode) -> Self {
        Self {
            id: vnode.id.to_string(),
            name: vnode.path.file_name().unwrap_or("").to_string(),
            path: vnode.path.to_string(),
            node_type: match vnode.node_type {
                NodeType::File => "file",
                NodeType::Directory => "directory",
                NodeType::SymLink => "symlink",
                NodeType::Document => "document",
            }
            .to_string(),
            size_bytes: vnode.size_bytes as u64,
            language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
            permissions: vnode.permissions,
            created_at: vnode.created_at,
            updated_at: vnode.updated_at,
            version: vnode.version,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryTree {
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub size_bytes: Option<u64>,
    pub children: Option<Vec<DirectoryTree>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_details_serialization() {
        let details = FileDetails {
            id: Uuid::new_v4().to_string(),
            name: "test.rs".to_string(),
            path: "/test.rs".to_string(),
            node_type: "file".to_string(),
            size_bytes: 1024,
            language: Some("rust".to_string()),
            permissions: Some(0o644),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("test.rs"));
    }

    #[test]
    fn test_directory_tree_serialization() {
        let tree = DirectoryTree {
            name: "root".to_string(),
            path: "/".to_string(),
            node_type: "directory".to_string(),
            size_bytes: None,
            children: Some(vec![
                DirectoryTree {
                    name: "file.txt".to_string(),
                    path: "/file.txt".to_string(),
                    node_type: "file".to_string(),
                    size_bytes: Some(100),
                    children: None,
                },
            ]),
        };

        let json = serde_json::to_string(&tree).unwrap();
        assert!(json.contains("root"));
        assert!(json.contains("file.txt"));
    }
}
