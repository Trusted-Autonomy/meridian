use anyhow::Result;

use crate::embed::{cosine, Embedder};
use crate::record::UsageRecord;
use crate::result::{ClassifiedRecord, KpiScore};
use crate::taxonomy::{Category, Kpi};

pub struct EmbeddingScorer {
    category_embeddings: Vec<(String, Vec<f32>)>,
    kpi_embeddings: Vec<(String, Vec<f32>)>,
}

impl EmbeddingScorer {
    /// Pre-embed all category and KPI descriptors. Call once at startup.
    pub fn build(embedder: &dyn Embedder, categories: &[Category], kpis: &[Kpi]) -> Result<Self> {
        let cat_texts: Vec<String> = categories
            .iter()
            .map(|c| {
                let kw = if c.keywords.is_empty() {
                    String::new()
                } else {
                    format!(". Keywords: {}", c.keywords.join(", "))
                };
                format!("{}: {}{}", c.label, c.description, kw)
            })
            .collect();

        let cat_refs: Vec<&str> = cat_texts.iter().map(String::as_str).collect();
        let cat_vecs = embedder.embed_batch(&cat_refs)?;
        let category_embeddings = categories
            .iter()
            .zip(cat_vecs)
            .map(|(c, v)| (c.id.clone(), v))
            .collect();

        let kpi_texts: Vec<String> = kpis
            .iter()
            .map(|k| format!("{}: {}", k.label, k.description))
            .collect();
        let kpi_refs: Vec<&str> = kpi_texts.iter().map(String::as_str).collect();
        let kpi_vecs = if kpi_refs.is_empty() {
            vec![]
        } else {
            embedder.embed_batch(&kpi_refs)?
        };
        let kpi_embeddings = kpis
            .iter()
            .zip(kpi_vecs)
            .map(|(k, v)| (k.id.clone(), v))
            .collect();

        Ok(Self {
            category_embeddings,
            kpi_embeddings,
        })
    }

    pub fn classify(
        &self,
        record: &UsageRecord,
        embedder: &dyn Embedder,
    ) -> Result<ClassifiedRecord> {
        let prompt_vec = embedder.embed_one(&record.title)?;
        Ok(self.classify_with_vec(record, &prompt_vec))
    }

    pub fn classify_batch(
        &self,
        records: &[UsageRecord],
        embedder: &dyn Embedder,
    ) -> Result<Vec<ClassifiedRecord>> {
        let titles: Vec<&str> = records.iter().map(|r| r.title.as_str()).collect();
        let prompt_vecs = embedder.embed_batch(&titles)?;
        Ok(records
            .iter()
            .zip(prompt_vecs)
            .map(|(record, prompt_vec)| self.classify_with_vec(record, &prompt_vec))
            .collect())
    }

    fn classify_with_vec(&self, record: &UsageRecord, prompt_vec: &[f32]) -> ClassifiedRecord {
        let (category_id, category_confidence) = self
            .category_embeddings
            .iter()
            .map(|(id, vec)| (id.as_str(), cosine(prompt_vec, vec)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, s)| (id.to_string(), s))
            .unwrap_or_else(|| ("unknown".to_string(), 0.0));

        let kpi_scores = self
            .kpi_embeddings
            .iter()
            .map(|(id, vec)| KpiScore {
                kpi_id: id.clone(),
                score: cosine(prompt_vec, vec),
            })
            .collect();

        ClassifiedRecord {
            record: record.clone(),
            category_id,
            category_confidence,
            kpi_scores,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::NullEmbedder;
    use crate::record::EffortUnits;
    use crate::taxonomy::Taxonomy;
    use chrono::Utc;

    fn make_record(title: &str) -> UsageRecord {
        UsageRecord {
            id: "test".into(),
            timestamp: Utc::now(),
            title: title.into(),
            effort: EffortUnits::Unknown,
            source: "test".into(),
            phase: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn builds_and_classifies_with_null_embedder() {
        let tax = Taxonomy::builtin();
        let embedder = NullEmbedder { dims: 4 };
        // NullEmbedder returns all zeros — all cosine similarities are 0,
        // so the first category wins. This just verifies no panic.
        let scorer = EmbeddingScorer::build(&embedder, &tax.categories, &tax.kpis).unwrap();
        let r = make_record("implement the auth feature");
        let result = scorer.classify(&r, &embedder).unwrap();
        assert!(!result.category_id.is_empty());
    }

    #[test]
    fn batch_classify_returns_one_per_record() {
        let tax = Taxonomy::builtin();
        let embedder = NullEmbedder { dims: 4 };
        let scorer = EmbeddingScorer::build(&embedder, &tax.categories, &tax.kpis).unwrap();
        let records = vec![make_record("code review"), make_record("write docs")];
        let results = scorer.classify_batch(&records, &embedder).unwrap();
        assert_eq!(results.len(), 2);
    }
}
