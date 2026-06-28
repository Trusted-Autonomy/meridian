use serde::{Deserialize, Serialize};

/// A measurable data point that tracks progress toward a KPI.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Metric {
    pub id: String,
    pub label: String,
    /// Unit of measurement: "hours", "count", "percent", "dollars", "days"
    pub unit: String,
    /// Target threshold expression: ">2", "<24", "100%", "<$50k"
    pub target: String,
    /// Data source: "ta_velocity", "git", "manual", "jira", "github"
    pub source: String,
    /// Collection frequency: "daily", "weekly", "monthly"
    pub frequency: String,
}
