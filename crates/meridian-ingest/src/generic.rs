use anyhow::{Context, Result};
use chrono::DateTime;
use meridian_core::record::{EffortUnits, UsageRecord};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Flexible JSONL record — `title` is required; everything else is optional.
/// Standalone input format for non-TA users.
#[derive(Debug, Deserialize)]
pub struct GenericRecord {
    pub id: Option<String>,
    pub title: String,
    pub timestamp: Option<String>,
    pub tokens_input: Option<u64>,
    pub tokens_output: Option<u64>,
    pub seconds: Option<u64>,
    pub source: Option<String>,
    pub phase: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

pub fn load_jsonl(path: &Path) -> Result<Vec<UsageRecord>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut records = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let entry: GenericRecord =
            serde_json::from_str(line).with_context(|| format!("line {}", lineno + 1))?;
        let timestamp = entry
            .timestamp
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);
        let effort = match (entry.tokens_input, entry.tokens_output, entry.seconds) {
            (Some(i), Some(o), _) => EffortUnits::Tokens {
                input: i,
                output: o,
            },
            (_, _, Some(s)) => EffortUnits::Seconds(s),
            _ => EffortUnits::Unknown,
        };
        let id = entry.id.unwrap_or_else(|| format!("record-{}", lineno));
        let metadata: HashMap<String, String> = entry
            .extra
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();
        records.push(UsageRecord {
            id,
            timestamp,
            title: entry.title,
            effort,
            source: entry.source.unwrap_or_else(|| "jsonl".to_string()),
            phase: entry.phase,
            metadata,
        });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_token_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"id":"r1","title":"build feature","tokens_input":5000,"tokens_output":1000,"timestamp":"2026-06-01T10:00:00Z"}}"#
        )
        .unwrap();
        let records = load_jsonl(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(
            records[0].effort,
            meridian_core::record::EffortUnits::Tokens {
                input: 5000,
                output: 1000
            }
        ));
    }

    #[test]
    fn load_seconds_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"title":"plan sprint","seconds":1800}}"#).unwrap();
        let records = load_jsonl(&path).unwrap();
        assert!(matches!(
            records[0].effort,
            meridian_core::record::EffortUnits::Seconds(1800)
        ));
    }

    #[test]
    fn skips_blank_lines_and_comments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("records.jsonl");
        std::fs::write(&path, "// comment\n\n{\"title\":\"real record\"}\n").unwrap();
        let records = load_jsonl(&path).unwrap();
        assert_eq!(records.len(), 1);
    }
}
