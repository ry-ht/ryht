//! Sangha consensus - harmonious agreement through iterative discussion

use super::*;

#[derive(Debug, Clone)]
pub struct SanghaConsensus {
    harmony_threshold: f32,
    max_rounds: usize,
    min_participation: f32,
}

impl Default for SanghaConsensus {
    fn default() -> Self {
        Self {
            harmony_threshold: 0.85,
            max_rounds: 5,
            min_participation: 0.8,
        }
    }
}

impl SanghaConsensus {
    fn calculate_harmony(&self, votes: &[Vote]) -> f32 {
        if votes.is_empty() {
            return 0.0;
        }

        let accept_count = votes
            .iter()
            .filter(|v| matches!(v.decision, Decision::Accept))
            .count() as f32;

        let total = votes.len() as f32;
        let alignment_ratio = accept_count / total;

        let avg_confidence: f32 = votes.iter().map(|v| v.confidence).sum::<f32>() / total;

        // Combine alignment and confidence
        alignment_ratio * 0.7 + avg_confidence * 0.3
    }
}

impl ConsensusStrategy for SanghaConsensus {
    fn name(&self) -> &str {
        "Sangha Consensus"
    }

    fn required_quorum(&self) -> f32 {
        self.min_participation
    }

    fn evaluate_votes(&self, votes: Vec<Vote>) -> Result<ConsensusResult> {
        let harmony = self.calculate_harmony(&votes);

        if harmony >= self.harmony_threshold {
            Ok(ConsensusResult::Harmonious {
                harmony_level: harmony,
                rounds: 1,
                votes,
            })
        } else {
            Ok(ConsensusResult::Failed {
                reason: format!("Harmony level {} below threshold {}", harmony, self.harmony_threshold),
                votes,
            })
        }
    }
}
