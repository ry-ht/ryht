//! Coordination patterns

use super::*;

pub struct StarPattern;
pub struct MeshPattern;
pub struct PipelinePattern;

impl CoordinationPattern for StarPattern {
    fn name(&self) -> &str {
        "Star"
    }

    fn description(&self) -> &str {
        "Central coordinator with worker agents"
    }
}

impl CoordinationPattern for MeshPattern {
    fn name(&self) -> &str {
        "Mesh"
    }

    fn description(&self) -> &str {
        "Fully connected agent network"
    }
}

impl CoordinationPattern for PipelinePattern {
    fn name(&self) -> &str {
        "Pipeline"
    }

    fn description(&self) -> &str {
        "Sequential processing pipeline"
    }
}
