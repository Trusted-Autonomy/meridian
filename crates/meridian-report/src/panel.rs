use meridian_core::panel::{charter_vector, score_title, PanelMember};
use meridian_core::record::UsageRecord;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PanelScore {
    pub role: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PanelResult {
    pub record: UsageRecord,
    pub scores: Vec<PanelScore>,
    /// Weighted average score across all panelists.
    pub consensus: f32,
    /// Standard deviation across scores — high value means panelists disagree.
    pub dissent: f32,
    /// Role that scored this record highest.
    pub champion: String,
    /// Role that scored this record lowest.
    pub skeptic: String,
}

pub struct PanelScorer {
    members_with_vecs: Vec<(PanelMember, std::collections::HashMap<String, f32>)>,
}

impl PanelScorer {
    pub fn new(panel: &[PanelMember]) -> Self {
        let members_with_vecs = panel
            .iter()
            .map(|m| (m.clone(), charter_vector(m)))
            .collect();
        Self { members_with_vecs }
    }

    pub fn score(&self, record: &UsageRecord) -> PanelResult {
        let scores: Vec<PanelScore> = self
            .members_with_vecs
            .iter()
            .map(|(m, vec)| PanelScore {
                role: m.role.clone(),
                score: score_title(&record.title, vec),
            })
            .collect();

        let total_weight: f32 = self.members_with_vecs.iter().map(|(m, _)| m.weight).sum();
        let consensus = if total_weight > 0.0 {
            self.members_with_vecs
                .iter()
                .zip(scores.iter())
                .map(|((m, _), s)| s.score * m.weight)
                .sum::<f32>()
                / total_weight
        } else {
            0.0
        };

        let n = scores.len() as f32;
        let mean = scores.iter().map(|s| s.score).sum::<f32>() / n.max(1.0);
        let variance = scores.iter().map(|s| (s.score - mean).powi(2)).sum::<f32>() / n.max(1.0);
        let dissent = variance.sqrt();

        let champion = scores
            .iter()
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.role.clone())
            .unwrap_or_default();

        let skeptic = scores
            .iter()
            .min_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.role.clone())
            .unwrap_or_default();

        PanelResult {
            record: record.clone(),
            scores,
            consensus,
            dissent,
            champion,
            skeptic,
        }
    }

    pub fn score_batch(&self, records: &[UsageRecord]) -> Vec<PanelResult> {
        records.iter().map(|r| self.score(r)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use meridian_core::panel::default_panel;
    use meridian_core::record::EffortUnits;

    fn record(title: &str) -> UsageRecord {
        UsageRecord {
            id: "t".into(),
            timestamp: Utc::now(),
            title: title.into(),
            effort: EffortUnits::Unknown,
            source: "test".into(),
            phase: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn panel_scorer_scores_relevant_title() {
        let panel = default_panel();
        let scorer = PanelScorer::new(&panel);
        let result = scorer.score(&record(
            "fix critical security vulnerability in authentication",
        ));

        let cto_score = result
            .scores
            .iter()
            .find(|s| s.role == "cto")
            .map(|s| s.score)
            .unwrap_or(0.0);
        let ceo_score = result
            .scores
            .iter()
            .find(|s| s.role == "ceo")
            .map(|s| s.score)
            .unwrap_or(0.0);

        assert!(
            cto_score > ceo_score,
            "CTO ({cto_score:.3}) should score higher than CEO ({ceo_score:.3}) for security work"
        );
    }

    #[test]
    fn consensus_is_weighted_average() {
        use meridian_core::panel::PanelMember;

        let panel = vec![
            PanelMember {
                role: "a".into(),
                charter: "alpha beta gamma".into(),
                weight: 1.0,
            },
            PanelMember {
                role: "b".into(),
                charter: "delta epsilon zeta".into(),
                weight: 1.0,
            },
        ];
        let scorer = PanelScorer::new(&panel);
        let result = scorer.score(&record("alpha beta gamma"));

        let s_a = result.scores.iter().find(|s| s.role == "a").unwrap().score;
        let s_b = result.scores.iter().find(|s| s.role == "b").unwrap().score;
        let expected = (s_a * 1.0 + s_b * 1.0) / 2.0;

        assert!(
            (result.consensus - expected).abs() < 1e-5,
            "consensus {:.4} != expected {:.4}",
            result.consensus,
            expected
        );
    }

    #[test]
    fn dissent_is_std_dev() {
        use meridian_core::panel::PanelMember;

        let panel = vec![
            PanelMember {
                role: "a".into(),
                charter: "alpha beta gamma".into(),
                weight: 1.0,
            },
            PanelMember {
                role: "b".into(),
                charter: "delta epsilon zeta".into(),
                weight: 1.0,
            },
        ];
        let scorer = PanelScorer::new(&panel);
        let result = scorer.score(&record("alpha beta gamma"));

        let scores: Vec<f32> = result.scores.iter().map(|s| s.score).collect();
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / scores.len() as f32;
        let expected_dissent = variance.sqrt();

        assert!(
            (result.dissent - expected_dissent).abs() < 1e-5,
            "dissent {:.4} != expected std dev {:.4}",
            result.dissent,
            expected_dissent
        );
    }
}
