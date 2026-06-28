use crate::record::UsageRecord;
use crate::result::{ClassifiedRecord, KpiScore};
use crate::taxonomy::{Category, Kpi};
use std::collections::HashMap;

fn tokenise(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(String::from)
        .collect()
}

fn tf_vector(tokens: &[String]) -> HashMap<String, f32> {
    let mut tf: HashMap<String, f32> = HashMap::new();
    for t in tokens {
        *tf.entry(t.clone()).or_insert(0.0) += 1.0;
    }
    tf
}

fn cosine(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    let dot: f32 = a
        .iter()
        .filter_map(|(k, v)| b.get(k).map(|bv| v * bv))
        .sum();
    let norm_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn category_vector(cat: &Category) -> HashMap<String, f32> {
    let mut tokens = tokenise(&cat.description);
    for kw in &cat.keywords {
        // Boost explicit keywords by repeating them twice.
        tokens.push(kw.clone());
        tokens.push(kw.clone());
    }
    tf_vector(&tokens)
}

fn kpi_vector(kpi: &Kpi) -> HashMap<String, f32> {
    tf_vector(&tokenise(&kpi.description))
}

/// Zero-cost bi-prism scorer: keyword TF cosine similarity for both
/// category classification (pass 1) and KPI alignment (pass 2).
pub struct KeywordScorer {
    category_vecs: Vec<(String, HashMap<String, f32>)>,
    kpi_vecs: Vec<(String, HashMap<String, f32>)>,
}

impl KeywordScorer {
    pub fn new(categories: &[Category], kpis: &[Kpi]) -> Self {
        let category_vecs = categories
            .iter()
            .map(|c| (c.id.clone(), category_vector(c)))
            .collect();
        let kpi_vecs = kpis.iter().map(|k| (k.id.clone(), kpi_vector(k))).collect();
        Self {
            category_vecs,
            kpi_vecs,
        }
    }

    pub fn classify(&self, record: &UsageRecord) -> ClassifiedRecord {
        let prompt_vec = tf_vector(&tokenise(&record.title));

        // Pass 1: category (nearest centroid).
        let (category_id, category_confidence) = self
            .category_vecs
            .iter()
            .map(|(id, vec)| (id.as_str(), cosine(&prompt_vec, vec)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, score)| (id.to_string(), score))
            .unwrap_or_else(|| ("unknown".to_string(), 0.0));

        // Pass 2: KPI alignment (independent of category).
        let kpi_scores = self
            .kpi_vecs
            .iter()
            .map(|(id, vec)| KpiScore {
                kpi_id: id.clone(),
                score: cosine(&prompt_vec, vec),
            })
            .collect();

        ClassifiedRecord {
            record: record.clone(),
            category_id,
            category_confidence,
            kpi_scores,
        }
    }

    pub fn classify_batch(&self, records: &[UsageRecord]) -> Vec<ClassifiedRecord> {
        records.iter().map(|r| self.classify(r)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::EffortUnits;
    use crate::taxonomy::Taxonomy;
    use chrono::Utc;

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
    fn classify_code_record() {
        let tax = Taxonomy::builtin();
        let scorer = KeywordScorer::new(&tax.categories, &tax.kpis);
        let r = scorer.classify(&record(
            "implement the new authentication feature with tests",
        ));
        assert_eq!(r.category_id, "code");
    }

    #[test]
    fn classify_docs_record() {
        let tax = Taxonomy::builtin();
        let scorer = KeywordScorer::new(&tax.categories, &tax.kpis);
        let r = scorer.classify(&record(
            "write changelog and update usage guide for onboarding",
        ));
        assert_eq!(r.category_id, "docs");
    }

    #[test]
    fn kpi_scores_empty_when_no_kpis() {
        let tax = Taxonomy::builtin();
        let scorer = KeywordScorer::new(&tax.categories, &tax.kpis);
        let r = scorer.classify(&record("deploy the release pipeline"));
        assert!(r.kpi_scores.is_empty());
    }

    #[test]
    fn cosine_identical_vectors() {
        let mut v = HashMap::new();
        v.insert("code".to_string(), 1.0f32);
        v.insert("test".to_string(), 1.0f32);
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let mut a = HashMap::new();
        a.insert("code".to_string(), 1.0f32);
        let mut b = HashMap::new();
        b.insert("finance".to_string(), 1.0f32);
        assert!((cosine(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn effort_points_tokens() {
        let e = EffortUnits::Tokens {
            input: 10_000,
            output: 2_000,
        };
        // 10 + 6 = 16
        assert!((e.effort_points() - 16.0).abs() < 1e-9);
    }

    #[test]
    fn effort_points_seconds() {
        assert!((EffortUnits::Seconds(3600).effort_points() - 3600.0).abs() < 1e-9);
    }
}
