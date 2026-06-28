use anyhow::{Context, Result};
use chrono::DateTime;
use meridian_core::record::{EffortUnits, UsageRecord};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct VelocityEntry {
    goal_id: String,
    title: String,
    plan_phase: Option<String>,
    outcome: Option<String>,
    started_at: Option<String>,
    #[serde(default)]
    total_seconds: u64,
}

pub struct TaSource;

impl TaSource {
    pub fn load_velocity(path: &Path) -> Result<Vec<UsageRecord>> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut records = Vec::new();
        for (lineno, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let entry: VelocityEntry =
                serde_json::from_str(line).with_context(|| format!("line {}", lineno + 1))?;
            let timestamp = entry
                .started_at
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now);
            let mut metadata = HashMap::new();
            if let Some(outcome) = &entry.outcome {
                metadata.insert("outcome".to_string(), outcome.clone());
            }
            records.push(UsageRecord {
                id: entry.goal_id,
                timestamp,
                title: entry.title,
                effort: EffortUnits::Seconds(entry.total_seconds),
                source: "ta".to_string(),
                phase: entry.plan_phase,
                metadata,
            });
        }
        Ok(records)
    }

    /// Walk up from `start` looking for `.ta/velocity-history.jsonl`.
    pub fn discover(start: &Path) -> Option<PathBuf> {
        let mut dir = start.to_path_buf();
        loop {
            let candidate = dir.join(".ta").join("velocity-history.jsonl");
            if candidate.exists() {
                return Some(candidate);
            }
            if !dir.pop() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_velocity_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("velocity-history.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"goal_id":"abc","title":"implement auth","plan_phase":"0.17.0","outcome":"applied","started_at":"2026-06-01T10:00:00Z","total_seconds":3600}}"#
        )
        .unwrap();
        let records = TaSource::load_velocity(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "implement auth");
        assert_eq!(records[0].phase.as_deref(), Some("0.17.0"));
        assert!(matches!(
            records[0].effort,
            meridian_core::record::EffortUnits::Seconds(3600)
        ));
    }

    #[test]
    fn discover_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let ta_dir = dir.path().join(".ta");
        std::fs::create_dir_all(&ta_dir).unwrap();
        let vel = ta_dir.join("velocity-history.jsonl");
        std::fs::write(&vel, "").unwrap();
        let sub = dir.path().join("sub").join("dir");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(TaSource::discover(&sub), Some(vel));
    }
}
