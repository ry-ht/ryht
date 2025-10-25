//! Conflict resolution strategies

use super::*;

#[derive(Debug, Clone)]
pub enum ConflictType {
    ResourceContention,
    MutualExclusion,
    CircularDependency,
    TemporalConflict,
}

#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    Priority,
    FirstComeFirstServed,
    Voting,
    Merge,
}

pub struct ConflictResolver {
    strategies: HashMap<ConflictType, ResolutionStrategy>,
}

impl ConflictResolver {
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        strategies.insert(ConflictType::ResourceContention, ResolutionStrategy::Priority);
        strategies.insert(ConflictType::MutualExclusion, ResolutionStrategy::Voting);

        Self { strategies }
    }

    pub fn resolve(&self, conflict_type: ConflictType) -> Option<&ResolutionStrategy> {
        self.strategies.get(&conflict_type)
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}
