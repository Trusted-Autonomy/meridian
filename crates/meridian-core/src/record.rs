use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortUnits {
    Seconds(u64),
    Tokens { input: u64, output: u64 },
    Unknown,
}

impl EffortUnits {
    /// Normalised cost-proxy in "effort points".
    /// Tokens: 1pt per 1k input + 3pt per 1k output (approximates cost ratio).
    /// Seconds: 1pt per second.
    pub fn effort_points(&self) -> f64 {
        match self {
            EffortUnits::Seconds(s) => *s as f64,
            EffortUnits::Tokens { input, output } => {
                (*input as f64 / 1000.0) + (*output as f64 / 1000.0 * 3.0)
            }
            EffortUnits::Unknown => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    /// Human-readable title or prompt excerpt — primary classification signal.
    pub title: String,
    pub effort: EffortUnits,
    pub source: String,
    /// Optional plan phase tag (e.g. "0.17.0.10" from TA).
    pub phase: Option<String>,
    pub metadata: HashMap<String, String>,
}
