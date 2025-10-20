//! Unique identifier types for Cortex entities.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A unique identifier for Cortex entities.
///
/// Uses UUIDv4 for globally unique, collision-resistant IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CortexId(Uuid);

impl CortexId {
    /// Create a new random ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an ID from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to a string representation
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    /// Parse from a string
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for CortexId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CortexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CortexId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<CortexId> for Uuid {
    fn from(id: CortexId) -> Self {
        id.0
    }
}

impl std::str::FromStr for CortexId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id1 = CortexId::new();
        let id2 = CortexId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_id_roundtrip() {
        let id = CortexId::new();
        let s = id.to_string();
        let parsed = CortexId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_id_serialization() {
        let id = CortexId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: CortexId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
