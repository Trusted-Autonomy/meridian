use meridian_config::AlignmentConfig;
use meridian_core::result::{ClassifiedRecord, KpiScore};
use serde::{Deserialize, Serialize};

/// KPI-alignment tier, derived from a record's best-fit KPI score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    High,
    Moderate,
    Low,
}

impl Tier {
    pub fn label(&self) -> &'static str {
        match self {
            Tier::High => "High",
            Tier::Moderate => "Moderate",
            Tier::Low => "Low",
        }
    }

    /// Case-insensitive parse of a `--tier` CLI value.
    pub fn parse(s: &str) -> Option<Tier> {
        match s.to_lowercase().as_str() {
            "high" => Some(Tier::High),
            "moderate" => Some(Tier::Moderate),
            "low" => Some(Tier::Low),
            _ => None,
        }
    }
}

/// The KPI a record fits best, and its score. Cosine similarity scores are
/// always in [0.0, 1.0], so folding from 0.0 also correctly handles the
/// empty-KPI-config case (no KPIs configured => best score is 0.0, tier Low)
/// without a special-cased empty check.
pub fn best_kpi(record: &ClassifiedRecord) -> Option<&KpiScore> {
    record.kpi_scores.iter().max_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub fn best_kpi_score(record: &ClassifiedRecord) -> f32 {
    record
        .kpi_scores
        .iter()
        .map(|s| s.score)
        .fold(0.0, f32::max)
}

/// Tier boundaries: `>= high_threshold` is High, `< low_threshold` is Low,
/// otherwise Moderate.
pub fn tier_for(score: f32, config: &AlignmentConfig) -> Tier {
    if score >= config.high_threshold {
        Tier::High
    } else if score < config.low_threshold {
        Tier::Low
    } else {
        Tier::Moderate
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TierSummary {
    pub tier: Tier,
    pub record_count: usize,
    pub total_effort: f64,
    /// `None` when no record in this tier carries a `credits_consumed` value
    /// (distinct from `Some(0.0)`, which means the source reported zero cost).
    pub total_credits: Option<f64>,
    pub avg_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlignmentSummary {
    /// Always High, Moderate, Low — in that order.
    pub tiers: Vec<TierSummary>,
}

pub fn summarise_alignment(
    records: &[ClassifiedRecord],
    config: &AlignmentConfig,
) -> AlignmentSummary {
    let scored: Vec<(&ClassifiedRecord, f32, Tier)> = records
        .iter()
        .map(|r| {
            let score = best_kpi_score(r);
            (r, score, tier_for(score, config))
        })
        .collect();

    let tiers = [Tier::High, Tier::Moderate, Tier::Low]
        .into_iter()
        .map(|tier| {
            let items: Vec<_> = scored.iter().filter(|(_, _, t)| *t == tier).collect();
            let record_count = items.len();
            // `f64::sum()` folds from -0.0 (IEEE 754), so an empty iterator
            // yields -0.0, not 0.0 — normalize so it never prints as "-0".
            let total_effort: f64 = items
                .iter()
                .map(|(r, _, _)| r.record.effort.effort_points())
                .sum::<f64>()
                + 0.0;
            let credits: Vec<f64> = items
                .iter()
                .filter_map(|(r, _, _)| r.record.credits_consumed)
                .collect();
            let total_credits = if credits.is_empty() {
                None
            } else {
                Some(credits.iter().sum::<f64>() + 0.0)
            };
            let avg_score = if record_count > 0 {
                items.iter().map(|(_, s, _)| *s).sum::<f32>() / record_count as f32
            } else {
                0.0
            };
            TierSummary {
                tier,
                record_count,
                total_effort,
                total_credits,
                avg_score,
            }
        })
        .collect();

    AlignmentSummary { tiers }
}

/// A single record's alignment detail, for `--detail`/`--tier`/`--min-score`/`--max-score` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredEntry {
    pub id: String,
    pub title: String,
    pub best_kpi_id: Option<String>,
    pub score: f32,
    pub tier: Tier,
    pub prompt: Option<String>,
    pub objective: Option<String>,
}

/// Every record's alignment detail, sorted ascending by score (worst-first) —
/// the whole point of this report is surfacing what needs review, so even
/// within the High tier the weakest/most-borderline entries come first.
pub fn scored_entries(records: &[ClassifiedRecord], config: &AlignmentConfig) -> Vec<ScoredEntry> {
    let mut entries: Vec<ScoredEntry> = records
        .iter()
        .map(|r| {
            let score = best_kpi_score(r);
            ScoredEntry {
                id: r.record.id.clone(),
                title: r.record.title.clone(),
                best_kpi_id: best_kpi(r).map(|k| k.kpi_id.clone()),
                score,
                tier: tier_for(score, config),
                prompt: r.record.prompt.clone(),
                objective: r.record.objective.clone(),
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    entries
}

/// A `--tier` or `--min-score`/`--max-score` filter, mutually exclusive at the CLI layer.
pub enum ScoreFilter {
    Tier(Tier),
    Range { min: Option<f32>, max: Option<f32> },
}

/// Apply an optional filter, then an optional count limit. `entries` must
/// already be sorted (as returned by `scored_entries`) — this preserves order.
pub fn filter_entries(
    entries: Vec<ScoredEntry>,
    filter: Option<&ScoreFilter>,
    limit: Option<usize>,
) -> Vec<ScoredEntry> {
    let mut filtered: Vec<ScoredEntry> = match filter {
        None => entries,
        Some(ScoreFilter::Tier(t)) => entries.into_iter().filter(|e| e.tier == *t).collect(),
        Some(ScoreFilter::Range { min, max }) => entries
            .into_iter()
            .filter(|e| min.is_none_or(|m| e.score >= m) && max.is_none_or(|m| e.score <= m))
            .collect(),
    };
    if let Some(n) = limit {
        filtered.truncate(n);
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use meridian_core::record::{EffortUnits, UsageRecord};

    fn config() -> AlignmentConfig {
        AlignmentConfig {
            low_threshold: 0.25,
            high_threshold: 0.6,
        }
    }

    fn record(
        id: &str,
        title: &str,
        scores: &[(&str, f32)],
        credits: Option<f64>,
    ) -> ClassifiedRecord {
        ClassifiedRecord {
            record: UsageRecord {
                id: id.to_string(),
                timestamp: Utc::now(),
                title: title.to_string(),
                effort: EffortUnits::Seconds(60),
                source: "test".into(),
                phase: None,
                metadata: Default::default(),
                prompt: None,
                objective: None,
                credits_consumed: credits,
            },
            category_id: "code".into(),
            category_confidence: 1.0,
            kpi_scores: scores
                .iter()
                .map(|(id, score)| KpiScore {
                    kpi_id: id.to_string(),
                    score: *score,
                })
                .collect(),
        }
    }

    #[test]
    fn tier_boundary_exactly_low_threshold_is_moderate_not_low() {
        let cfg = config();
        assert_eq!(tier_for(0.25, &cfg), Tier::Moderate);
        assert_eq!(tier_for(0.2499, &cfg), Tier::Low);
    }

    #[test]
    fn tier_boundary_exactly_high_threshold_is_high() {
        let cfg = config();
        assert_eq!(tier_for(0.6, &cfg), Tier::High);
        assert_eq!(tier_for(0.5999, &cfg), Tier::Moderate);
    }

    #[test]
    fn best_kpi_score_is_max_across_kpis() {
        let r = record("a", "t", &[("rev", 0.1), ("eng", 0.7), ("risk", 0.3)], None);
        assert_eq!(best_kpi_score(&r), 0.7);
        assert_eq!(best_kpi(&r).unwrap().kpi_id, "eng");
    }

    #[test]
    fn best_kpi_score_with_no_kpis_is_zero_not_panic() {
        let r = record("a", "t", &[], None);
        assert_eq!(best_kpi_score(&r), 0.0);
        assert!(best_kpi(&r).is_none());
    }

    #[test]
    fn summarise_alignment_aggregates_count_effort_credits_avg_per_tier() {
        let records = vec![
            record("a", "high one", &[("k", 0.8)], Some(2.0)),
            record("b", "high two", &[("k", 0.65)], Some(3.0)),
            record("c", "moderate one", &[("k", 0.4)], None),
            record("d", "low one", &[("k", 0.1)], None),
        ];
        let cfg = config();
        let summary = summarise_alignment(&records, &cfg);
        let high = summary.tiers.iter().find(|t| t.tier == Tier::High).unwrap();
        assert_eq!(high.record_count, 2);
        assert_eq!(high.total_effort, 120.0);
        assert_eq!(high.total_credits, Some(5.0));
        assert!((high.avg_score - 0.725).abs() < 1e-6);

        let moderate = summary
            .tiers
            .iter()
            .find(|t| t.tier == Tier::Moderate)
            .unwrap();
        assert_eq!(moderate.record_count, 1);
        assert_eq!(moderate.total_credits, None);

        let low = summary.tiers.iter().find(|t| t.tier == Tier::Low).unwrap();
        assert_eq!(low.record_count, 1);
    }

    #[test]
    fn empty_tier_total_effort_is_positive_zero_not_negative_zero() {
        // f64::sum() on an empty iterator folds from -0.0 (IEEE 754) — assert
        // this is normalized so an empty tier never renders as "-0".
        let records = vec![record("a", "only high", &[("k", 0.9)], None)];
        let summary = summarise_alignment(&records, &config());
        let moderate = summary
            .tiers
            .iter()
            .find(|t| t.tier == Tier::Moderate)
            .unwrap();
        assert_eq!(moderate.record_count, 0);
        assert!(moderate.total_effort.is_sign_positive());
        assert_eq!(moderate.total_effort, 0.0);
    }

    #[test]
    fn scored_entries_sorted_ascending_by_score() {
        let records = vec![
            record("a", "high", &[("k", 0.8)], None),
            record("b", "low", &[("k", 0.1)], None),
            record("c", "moderate", &[("k", 0.4)], None),
        ];
        let entries = scored_entries(&records, &config());
        let scores: Vec<f32> = entries.iter().map(|e| e.score).collect();
        assert_eq!(scores, vec![0.1, 0.4, 0.8]);
    }

    #[test]
    fn filter_entries_by_tier() {
        let records = vec![
            record("a", "high", &[("k", 0.8)], None),
            record("b", "low", &[("k", 0.1)], None),
        ];
        let entries = scored_entries(&records, &config());
        let filtered = filter_entries(entries, Some(&ScoreFilter::Tier(Tier::Low)), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "b");
    }

    #[test]
    fn filter_entries_by_score_range() {
        let records = vec![
            record("a", "one", &[("k", 0.27)], None),
            record("b", "two", &[("k", 0.29)], None),
            record("c", "three", &[("k", 0.5)], None),
        ];
        let entries = scored_entries(&records, &config());
        let filtered = filter_entries(
            entries,
            Some(&ScoreFilter::Range {
                min: Some(0.25),
                max: Some(0.3),
            }),
            None,
        );
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_entries_respects_limit_after_filtering() {
        let records = vec![
            record("a", "one", &[("k", 0.1)], None),
            record("b", "two", &[("k", 0.2)], None),
            record("c", "three", &[("k", 0.05)], None),
        ];
        let entries = scored_entries(&records, &config());
        let filtered = filter_entries(entries, None, Some(2));
        assert_eq!(filtered.len(), 2);
        // still ascending, so the two lowest scores
        assert_eq!(filtered[0].id, "c");
        assert_eq!(filtered[1].id, "a");
    }
}
