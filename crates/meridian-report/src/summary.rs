use meridian_core::result::ClassifiedRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CategorySummary {
    pub category_id: String,
    pub record_count: usize,
    pub total_effort: f64,
    pub effort_pct: f64,
    /// Average KPI alignment scores (0.0–1.0) for all records in this category.
    pub kpi_alignment: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_records: usize,
    pub total_effort: f64,
    /// Sorted by total_effort descending.
    pub by_category: Vec<CategorySummary>,
}

pub fn summarise(records: &[ClassifiedRecord]) -> AnalysisSummary {
    let total_effort: f64 = records
        .iter()
        .map(|r| r.record.effort.effort_points())
        .sum();
    let mut by_cat: HashMap<String, Vec<&ClassifiedRecord>> = HashMap::new();
    for r in records {
        by_cat.entry(r.category_id.clone()).or_default().push(r);
    }

    let mut by_category: Vec<CategorySummary> = by_cat
        .iter()
        .map(|(cat_id, items)| {
            let effort: f64 = items.iter().map(|r| r.record.effort.effort_points()).sum();
            let effort_pct = if total_effort > 0.0 {
                effort / total_effort * 100.0
            } else {
                0.0
            };

            let mut kpi_sums: HashMap<String, f32> = HashMap::new();
            let mut kpi_counts: HashMap<String, usize> = HashMap::new();
            for item in items {
                for ks in &item.kpi_scores {
                    *kpi_sums.entry(ks.kpi_id.clone()).or_default() += ks.score;
                    *kpi_counts.entry(ks.kpi_id.clone()).or_default() += 1;
                }
            }
            let kpi_alignment = kpi_sums
                .iter()
                .map(|(id, sum)| {
                    let count = *kpi_counts.get(id).unwrap_or(&1) as f32;
                    (id.clone(), sum / count)
                })
                .collect();

            CategorySummary {
                category_id: cat_id.clone(),
                record_count: items.len(),
                total_effort: effort,
                effort_pct,
                kpi_alignment,
            }
        })
        .collect();

    by_category.sort_by(|a, b| {
        b.total_effort
            .partial_cmp(&a.total_effort)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    AnalysisSummary {
        total_records: records.len(),
        total_effort,
        by_category,
    }
}
