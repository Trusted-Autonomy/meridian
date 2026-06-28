use crate::record::UsageRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiScore {
    pub kpi_id: String,
    /// 0.0–1.0 cosine similarity to the KPI description.
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedRecord {
    pub record: UsageRecord,
    pub category_id: String,
    /// 0.0–1.0 confidence in the category assignment.
    pub category_confidence: f32,
    pub kpi_scores: Vec<KpiScore>,
}
