use super::types::{KnowledgeLevel, LinkId, LinkTarget, LinkType, SemanticLink, ValidationStatus};
use crate::storage::{serialize, deserialize, Storage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for storing and querying semantic links
#[async_trait]
pub trait LinksStorage: Send + Sync {
    /// Add a new link to the storage
    async fn add_link(&self, link: &SemanticLink) -> Result<()>;

    /// Remove a link by ID
    async fn remove_link(&self, link_id: &LinkId) -> Result<()>;

    /// Get a link by ID
    async fn get_link(&self, link_id: &LinkId) -> Result<Option<SemanticLink>>;

    /// Update an existing link
    async fn update_link(&self, link: &SemanticLink) -> Result<()>;

    /// Find all links from a source
    async fn find_links_from_source(&self, source: &LinkTarget) -> Result<Vec<SemanticLink>>;

    /// Find all links to a target
    async fn find_links_to_target(&self, target: &LinkTarget) -> Result<Vec<SemanticLink>>;

    /// Find links by type
    async fn find_links_by_type(&self, link_type: LinkType) -> Result<Vec<SemanticLink>>;

    /// Find links by type from a specific source
    async fn find_links_by_type_from_source(
        &self,
        link_type: LinkType,
        source: &LinkTarget,
    ) -> Result<Vec<SemanticLink>>;

    /// Find links by type to a specific target
    async fn find_links_by_type_to_target(
        &self,
        link_type: LinkType,
        target: &LinkTarget,
    ) -> Result<Vec<SemanticLink>>;

    /// Get bidirectional links for an entity
    async fn get_bidirectional_links(&self, entity: &LinkTarget) -> Result<BidirectionalLinks>;

    /// Find links between two knowledge levels
    async fn find_cross_level_links(
        &self,
        source_level: KnowledgeLevel,
        target_level: KnowledgeLevel,
    ) -> Result<Vec<SemanticLink>>;

    /// Find broken links
    async fn find_broken_links(&self) -> Result<Vec<SemanticLink>>;

    /// Validate and update link status
    async fn validate_link(&self, link_id: &LinkId, status: ValidationStatus) -> Result<()>;

    /// Count total links
    async fn count_links(&self) -> Result<usize>;

    /// Get link statistics
    async fn get_statistics(&self) -> Result<LinkStatistics>;
}

/// Bidirectional links for an entity
#[derive(Debug, Clone)]
pub struct BidirectionalLinks {
    /// Links where this entity is the source
    pub outgoing: Vec<SemanticLink>,
    /// Links where this entity is the target
    pub incoming: Vec<SemanticLink>,
}

/// Statistics about links in the storage
#[derive(Debug, Clone)]
pub struct LinkStatistics {
    pub total_links: usize,
    pub by_type: HashMap<String, usize>,
    pub by_level: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub average_confidence: f32,
}

/// RocksDB implementation of LinksStorage
pub struct RocksDBLinksStorage {
    storage: Arc<dyn Storage>,
}

impl RocksDBLinksStorage {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Generate key for a link
    fn link_key(link_id: &LinkId) -> Vec<u8> {
        format!("link:{}", link_id.as_str()).into_bytes()
    }

    /// Generate key for source index
    fn source_index_key(source: &LinkTarget) -> Vec<u8> {
        format!("idx_source:{}", source.key()).into_bytes()
    }

    /// Generate key for target index
    fn target_index_key(target: &LinkTarget) -> Vec<u8> {
        format!("idx_target:{}", target.key()).into_bytes()
    }

    /// Generate key for type-source index
    fn type_source_index_key(link_type: LinkType, source: &LinkTarget) -> Vec<u8> {
        format!("idx_type:{}:{}", link_type.as_str(), source.key()).into_bytes()
    }

    /// Generate key for type-target index
    fn type_target_index_key(link_type: LinkType, target: &LinkTarget) -> Vec<u8> {
        format!("idx_type_reverse:{}:{}", link_type.as_str(), target.key()).into_bytes()
    }

    /// Generate key for type index
    fn type_index_key(link_type: LinkType) -> Vec<u8> {
        format!("idx_type_only:{}", link_type.as_str()).into_bytes()
    }

    /// Generate key prefix for type index
    #[allow(dead_code)]
    fn type_index_prefix(link_type: LinkType) -> Vec<u8> {
        format!("idx_type:{}:", link_type.as_str()).into_bytes()
    }

    /// Generate key for cross-level index
    fn cross_level_key(source_level: KnowledgeLevel, target_level: KnowledgeLevel) -> Vec<u8> {
        format!(
            "idx_cross:{}:{}",
            source_level.as_str(),
            target_level.as_str()
        )
        .into_bytes()
    }

    /// Generate key for broken links index
    fn broken_links_key() -> Vec<u8> {
        b"idx_broken".to_vec()
    }

    /// Generate key for validation status index
    fn validation_status_key(status: ValidationStatus) -> Vec<u8> {
        format!("idx_status:{}", status.as_str()).into_bytes()
    }

    /// Add link ID to an index
    async fn add_to_index(&self, index_key: Vec<u8>, link_id: &LinkId) -> Result<()> {
        let mut link_ids = self.get_link_ids_from_index(&index_key).await?;
        if !link_ids.contains(&link_id.as_str().to_string()) {
            link_ids.push(link_id.as_str().to_string());
            let serialized = serialize(&link_ids)?;
            self.storage.put(&index_key, &serialized).await?;
        }
        Ok(())
    }

    /// Remove link ID from an index
    async fn remove_from_index(&self, index_key: Vec<u8>, link_id: &LinkId) -> Result<()> {
        let mut link_ids = self.get_link_ids_from_index(&index_key).await?;
        link_ids.retain(|id| id != link_id.as_str());
        if link_ids.is_empty() {
            self.storage.delete(&index_key).await?;
        } else {
            let serialized = serialize(&link_ids)?;
            self.storage.put(&index_key, &serialized).await?;
        }
        Ok(())
    }

    /// Get link IDs from an index
    async fn get_link_ids_from_index(&self, index_key: &[u8]) -> Result<Vec<String>> {
        match self.storage.get(index_key).await? {
            Some(data) => deserialize(&data),
            None => Ok(Vec::new()),
        }
    }

    /// Get links by IDs
    async fn get_links_by_ids(&self, link_ids: &[String]) -> Result<Vec<SemanticLink>> {
        let mut links = Vec::new();
        for link_id_str in link_ids {
            let link_id = LinkId::from_string(link_id_str.clone());
            if let Some(link) = self.get_link(&link_id).await? {
                links.push(link);
            }
        }
        Ok(links)
    }

    /// Update all indices for a link
    async fn update_indices(&self, link: &SemanticLink, remove: bool) -> Result<()> {
        let ops = if remove {
            vec![
                (Self::source_index_key(&link.source), &link.id),
                (Self::target_index_key(&link.target), &link.id),
                (
                    Self::type_source_index_key(link.link_type, &link.source),
                    &link.id,
                ),
                (
                    Self::type_target_index_key(link.link_type, &link.target),
                    &link.id,
                ),
                (Self::type_index_key(link.link_type), &link.id),
                (
                    Self::cross_level_key(link.source.level, link.target.level),
                    &link.id,
                ),
                (Self::validation_status_key(link.validation_status), &link.id),
            ]
        } else {
            vec![
                (Self::source_index_key(&link.source), &link.id),
                (Self::target_index_key(&link.target), &link.id),
                (
                    Self::type_source_index_key(link.link_type, &link.source),
                    &link.id,
                ),
                (
                    Self::type_target_index_key(link.link_type, &link.target),
                    &link.id,
                ),
                (Self::type_index_key(link.link_type), &link.id),
                (
                    Self::cross_level_key(link.source.level, link.target.level),
                    &link.id,
                ),
                (Self::validation_status_key(link.validation_status), &link.id),
            ]
        };

        for (index_key, link_id) in ops {
            if remove {
                self.remove_from_index(index_key, link_id).await?;
            } else {
                self.add_to_index(index_key, link_id).await?;
            }
        }

        // Handle broken links index separately
        if link.validation_status == ValidationStatus::Broken {
            if !remove {
                self.add_to_index(Self::broken_links_key(), &link.id)
                    .await?;
            }
        } else if remove {
            self.remove_from_index(Self::broken_links_key(), &link.id)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl LinksStorage for RocksDBLinksStorage {
    async fn add_link(&self, link: &SemanticLink) -> Result<()> {
        // Validate link
        if link.confidence < 0.0 || link.confidence > 1.0 {
            return Err(anyhow!("Invalid confidence value: {}", link.confidence));
        }

        // Serialize and store the link
        let key = Self::link_key(&link.id);
        let value = serialize(link)?;
        self.storage.put(&key, &value).await?;

        // Update indices
        self.update_indices(link, false).await?;

        Ok(())
    }

    async fn remove_link(&self, link_id: &LinkId) -> Result<()> {
        // Get the link first to update indices
        let link = self
            .get_link(link_id)
            .await?
            .ok_or_else(|| anyhow!("Link not found: {}", link_id))?;

        // Remove from indices
        self.update_indices(&link, true).await?;

        // Remove the link itself
        let key = Self::link_key(link_id);
        self.storage.delete(&key).await?;

        Ok(())
    }

    async fn get_link(&self, link_id: &LinkId) -> Result<Option<SemanticLink>> {
        let key = Self::link_key(link_id);
        match self.storage.get(&key).await? {
            Some(data) => Ok(Some(deserialize(&data)?)),
            None => Ok(None),
        }
    }

    async fn update_link(&self, link: &SemanticLink) -> Result<()> {
        // Get old link to update indices if needed
        if let Some(old_link) = self.get_link(&link.id).await? {
            // Remove old indices
            self.update_indices(&old_link, true).await?;
        }

        // Add new link (this will also update indices)
        self.add_link(link).await?;

        Ok(())
    }

    async fn find_links_from_source(&self, source: &LinkTarget) -> Result<Vec<SemanticLink>> {
        let index_key = Self::source_index_key(source);
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn find_links_to_target(&self, target: &LinkTarget) -> Result<Vec<SemanticLink>> {
        let index_key = Self::target_index_key(target);
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn find_links_by_type(&self, link_type: LinkType) -> Result<Vec<SemanticLink>> {
        let index_key = Self::type_index_key(link_type);
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn find_links_by_type_from_source(
        &self,
        link_type: LinkType,
        source: &LinkTarget,
    ) -> Result<Vec<SemanticLink>> {
        let index_key = Self::type_source_index_key(link_type, source);
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn find_links_by_type_to_target(
        &self,
        link_type: LinkType,
        target: &LinkTarget,
    ) -> Result<Vec<SemanticLink>> {
        let index_key = Self::type_target_index_key(link_type, target);
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn get_bidirectional_links(&self, entity: &LinkTarget) -> Result<BidirectionalLinks> {
        let outgoing = self.find_links_from_source(entity).await?;
        let incoming = self.find_links_to_target(entity).await?;

        Ok(BidirectionalLinks { outgoing, incoming })
    }

    async fn find_cross_level_links(
        &self,
        source_level: KnowledgeLevel,
        target_level: KnowledgeLevel,
    ) -> Result<Vec<SemanticLink>> {
        let index_key = Self::cross_level_key(source_level, target_level);
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn find_broken_links(&self) -> Result<Vec<SemanticLink>> {
        let index_key = Self::broken_links_key();
        let link_ids = self.get_link_ids_from_index(&index_key).await?;
        self.get_links_by_ids(&link_ids).await
    }

    async fn validate_link(&self, link_id: &LinkId, status: ValidationStatus) -> Result<()> {
        let mut link = self
            .get_link(link_id)
            .await?
            .ok_or_else(|| anyhow!("Link not found: {}", link_id))?;

        let old_status = link.validation_status;
        link.validate(status);

        // If status changed, update indices
        if old_status != status {
            // Remove from old status index
            self.remove_from_index(Self::validation_status_key(old_status), link_id)
                .await?;

            // Add to new status index
            self.add_to_index(Self::validation_status_key(status), link_id)
                .await?;

            // Handle broken links index
            if status == ValidationStatus::Broken {
                self.add_to_index(Self::broken_links_key(), link_id)
                    .await?;
            } else if old_status == ValidationStatus::Broken {
                self.remove_from_index(Self::broken_links_key(), link_id)
                    .await?;
            }
        }

        // Update the link
        let key = Self::link_key(link_id);
        let value = serialize(&link)?;
        self.storage.put(&key, &value).await?;

        Ok(())
    }

    async fn count_links(&self) -> Result<usize> {
        let prefix = b"link:";
        let keys = self.storage.get_keys_with_prefix(prefix).await?;
        Ok(keys.len())
    }

    async fn get_statistics(&self) -> Result<LinkStatistics> {
        let prefix = b"link:";
        let keys = self.storage.get_keys_with_prefix(prefix).await?;

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_level: HashMap<String, usize> = HashMap::new();
        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut total_confidence = 0.0;
        let mut count = 0;

        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                if let Ok(link) = deserialize::<SemanticLink>(&data) {
                    *by_type.entry(link.link_type.as_str().to_string()).or_insert(0) += 1;
                    *by_level
                        .entry(link.source.level.as_str().to_string())
                        .or_insert(0) += 1;
                    *by_status
                        .entry(link.validation_status.as_str().to_string())
                        .or_insert(0) += 1;
                    total_confidence += link.confidence;
                    count += 1;
                }
            }
        }

        let average_confidence = if count > 0 {
            total_confidence / count as f32
        } else {
            0.0
        };

        Ok(LinkStatistics {
            total_links: count,
            by_type,
            by_level,
            by_status,
            average_confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::ExtractionMethod;
    use crate::storage::MemoryStorage;
    use tempfile::TempDir;

    async fn create_test_storage() -> (RocksDBLinksStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MemoryStorage::new());
        let links_storage = RocksDBLinksStorage::new(storage);
        (links_storage, temp_dir)
    }

    #[tokio::test]
    async fn test_add_and_get_link() {
        let (storage, _temp) = create_test_storage().await;

        let source = LinkTarget::spec("spec.md#feature".to_string());
        let target = LinkTarget::code("Implementation".to_string());
        let link = SemanticLink::new(
            LinkType::ImplementedBy,
            source,
            target,
            0.95,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&link).await.unwrap();

        let retrieved = storage.get_link(&link.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, link.id);
    }

    #[tokio::test]
    async fn test_find_links_from_source() {
        let (storage, _temp) = create_test_storage().await;

        let source = LinkTarget::code("MyClass".to_string());
        let target1 = LinkTarget::docs("docs/myclass.md".to_string());
        let target2 = LinkTarget::tests("tests/myclass.spec.ts".to_string());

        let link1 = SemanticLink::new(
            LinkType::DocumentedIn,
            source.clone(),
            target1,
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        let link2 = SemanticLink::new(
            LinkType::TestedBy,
            source.clone(),
            target2,
            0.95,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&link1).await.unwrap();
        storage.add_link(&link2).await.unwrap();

        let links = storage.find_links_from_source(&source).await.unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_find_links_by_type() {
        let (storage, _temp) = create_test_storage().await;

        let link1 = SemanticLink::new(
            LinkType::ImplementedBy,
            LinkTarget::spec("spec1".to_string()),
            LinkTarget::code("code1".to_string()),
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        let link2 = SemanticLink::new(
            LinkType::ImplementedBy,
            LinkTarget::spec("spec2".to_string()),
            LinkTarget::code("code2".to_string()),
            0.95,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&link1).await.unwrap();
        storage.add_link(&link2).await.unwrap();

        let links = storage
            .find_links_by_type(LinkType::ImplementedBy)
            .await
            .unwrap();
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_bidirectional_links() {
        let (storage, _temp) = create_test_storage().await;

        let entity = LinkTarget::code("MyClass".to_string());

        let outgoing = SemanticLink::new(
            LinkType::DocumentedIn,
            entity.clone(),
            LinkTarget::docs("docs.md".to_string()),
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        let incoming = SemanticLink::new(
            LinkType::Tests,
            LinkTarget::tests("test.ts".to_string()),
            entity.clone(),
            0.95,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&outgoing).await.unwrap();
        storage.add_link(&incoming).await.unwrap();

        let bi_links = storage.get_bidirectional_links(&entity).await.unwrap();
        assert_eq!(bi_links.outgoing.len(), 1);
        assert_eq!(bi_links.incoming.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_link() {
        let (storage, _temp) = create_test_storage().await;

        let link = SemanticLink::new(
            LinkType::ImplementedBy,
            LinkTarget::spec("spec".to_string()),
            LinkTarget::code("code".to_string()),
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&link).await.unwrap();
        assert!(storage.get_link(&link.id).await.unwrap().is_some());

        storage.remove_link(&link.id).await.unwrap();
        assert!(storage.get_link(&link.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_validate_link() {
        let (storage, _temp) = create_test_storage().await;

        let link = SemanticLink::new(
            LinkType::ImplementedBy,
            LinkTarget::spec("spec".to_string()),
            LinkTarget::code("code".to_string()),
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&link).await.unwrap();

        storage
            .validate_link(&link.id, ValidationStatus::Valid)
            .await
            .unwrap();

        let updated = storage.get_link(&link.id).await.unwrap().unwrap();
        assert_eq!(updated.validation_status, ValidationStatus::Valid);
        assert!(updated.last_validated.is_some());
    }

    #[tokio::test]
    async fn test_statistics() {
        let (storage, _temp) = create_test_storage().await;

        let link1 = SemanticLink::new(
            LinkType::ImplementedBy,
            LinkTarget::spec("spec1".to_string()),
            LinkTarget::code("code1".to_string()),
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        let link2 = SemanticLink::new(
            LinkType::DocumentedIn,
            LinkTarget::code("code2".to_string()),
            LinkTarget::docs("docs2".to_string()),
            0.95,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        storage.add_link(&link1).await.unwrap();
        storage.add_link(&link2).await.unwrap();

        let stats = storage.get_statistics().await.unwrap();
        assert_eq!(stats.total_links, 2);
        assert!(stats.by_type.contains_key("implemented_by"));
        assert!(stats.by_type.contains_key("documented_in"));
    }
}
