//! Voting mechanisms

use super::*;

#[derive(Debug, Clone)]
pub struct SimpleMajority {
    threshold: f32,
    min_participation: f32,
}

impl Default for SimpleMajority {
    fn default() -> Self {
        Self {
            threshold: 0.51,
            min_participation: 0.5,
        }
    }
}

impl ConsensusStrategy for SimpleMajority {
    fn name(&self) -> &str {
        "Simple Majority"
    }

    fn required_quorum(&self) -> f32 {
        self.min_participation
    }

    fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let total = votes.len() as f32;
        let accepts = votes
            .iter()
            .filter(|v| v.decision == Decision::Accept)
            .count() as f32;

        let support = if total > 0.0 { accepts / total } else { 0.0 };

        if support > self.threshold {
            Ok(ConsensusResult::Accepted {
                support,
                votes,
                unanimous: support >= 0.99,
            })
        } else {
            Ok(ConsensusResult::Rejected { support, votes })
        }
    }
}
