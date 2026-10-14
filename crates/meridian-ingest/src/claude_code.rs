// Reads Claude Code session transcripts from ~/.claude/projects/
//
// Each session file: ~/.claude/projects/<project-hash>/<uuid>.jsonl
// Produces one UsageRecord per session with:
//   - title: ai-title record content, or first user message (first 120 chars)
//   - effort: sum of input+cache_creation tokens (input) and output tokens across all assistant turns,
//     counting each API message once (streamed rows repeat a `message.id`; the last row wins) and
//     skipping `<synthetic>` rows the client inserts without calling the API
//   - metadata["cache_read_tokens"]: cache-read tokens, kept out of `effort` (priced ~10x lower) but
//     recorded because they dominate long sessions
//   - source: "claude-code"
//   - metadata["project"]: decoded path from directory name

use anyhow::{Context, Result};
use chrono::DateTime;
use meridian_core::record::{EffortUnits, UsageRecord};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Decode a project hash directory name back to a filesystem path.
/// `-Users-michael-development-foo` → `/Users/michael/development/foo`
fn decode_project_path(dir_name: &str) -> String {
    if let Some(rest) = dir_name.strip_prefix('-') {
        format!("/{}", rest.replace('-', "/"))
    } else {
        dir_name.replace('-', "/")
    }
}

/// Default Claude Code projects directory: `~/.claude/projects/`
pub fn default_projects_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".claude")
        .join("projects")
}

/// Load all sessions from every project in a Claude Code projects directory.
pub fn load_all(projects_dir: &Path) -> Result<Vec<UsageRecord>> {
    let mut records = Vec::new();
    let entries = std::fs::read_dir(projects_dir)
        .with_context(|| format!("reading {}", projects_dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let project_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let project_path = decode_project_path(&project_name);
            let sessions = load_project(&path, &project_path)?;
            records.extend(sessions);
        }
    }
    Ok(records)
}

/// Load all sessions from a single project directory.
pub fn load_project(project_dir: &Path, project_path: &str) -> Result<Vec<UsageRecord>> {
    let mut records = Vec::new();
    let entries = std::fs::read_dir(project_dir)
        .with_context(|| format!("reading {}", project_dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            if let Ok(Some(record)) = load_session(&path, project_path) {
                records.push(record);
            }
        }
    }
    Ok(records)
}

/// Load a single session JSONL file into a UsageRecord.
/// Returns `None` if the session has no user turns (system-only or empty file).
pub fn load_session(session_file: &Path, project_path: &str) -> Result<Option<UsageRecord>> {
    let content = std::fs::read_to_string(session_file)
        .with_context(|| format!("reading {}", session_file.display()))?;

    let mut title: Option<String> = None;
    let mut timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    // Usage keyed by message id; the last row for an id wins. Rows without an id are summed as-is.
    let mut by_id: HashMap<String, Usage> = HashMap::new();
    let mut no_id: Vec<Usage> = Vec::new();
    let mut has_user_turn = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(record): Result<Value, _> = serde_json::from_str(line) else {
            continue;
        };

        let record_type = record.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match record_type {
            "ai-title" => {
                // title may be at root "title", or nested in message.content (str or array)
                let t = record
                    .get("title")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        record
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                    })
                    .or_else(|| {
                        record
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|item| item.get("text"))
                            .and_then(|v| v.as_str())
                    });
                if let Some(t) = t {
                    let t = t.trim();
                    if !t.is_empty() {
                        title = Some(t.to_string());
                    }
                }
            }
            "user" => {
                has_user_turn = true;
                if title.is_none() {
                    if let Some(text) = extract_user_text(&record) {
                        title = Some(text.chars().take(120).collect());
                    }
                }
                if timestamp.is_none() {
                    if let Some(ts) = record.get("timestamp").and_then(|v| v.as_str()) {
                        timestamp = DateTime::parse_from_rfc3339(ts)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc));
                    }
                }
            }
            "assistant" => {
                let message = record.get("message");
                if message
                    .and_then(|m| m.get("model"))
                    .and_then(|v| v.as_str())
                    == Some("<synthetic>")
                {
                    continue;
                }
                if let Some(usage) = message.and_then(|m| m.get("usage")) {
                    let u = Usage::from_value(usage);
                    match message.and_then(|m| m.get("id")).and_then(|v| v.as_str()) {
                        Some(id) => {
                            by_id.insert(id.to_string(), u);
                        }
                        None => no_id.push(u),
                    }
                }
            }
            _ => {}
        }
    }

    for u in by_id.values().chain(no_id.iter()) {
        total_input += u.input + u.cache_creation;
        total_output += u.output;
        total_cache_read += u.cache_read;
    }

    if !has_user_turn {
        return Ok(None);
    }

    let session_id = session_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut metadata = HashMap::new();
    metadata.insert("project".to_string(), project_path.to_string());
    metadata.insert("session_id".to_string(), session_id.clone());
    metadata.insert(
        "cache_read_tokens".to_string(),
        total_cache_read.to_string(),
    );

    Ok(Some(UsageRecord {
        id: session_id,
        timestamp: timestamp.unwrap_or_else(chrono::Utc::now),
        title: title.unwrap_or_else(|| "(untitled session)".to_string()),
        effort: EffortUnits::Tokens {
            input: total_input,
            output: total_output,
        },
        source: "claude-code".to_string(),
        phase: None,
        metadata,
        prompt: None,
        objective: None,
        credits_consumed: None,
    }))
}

/// Token usage reported on one assistant message.
struct Usage {
    input: u64,
    cache_creation: u64,
    cache_read: u64,
    output: u64,
}

impl Usage {
    fn from_value(usage: &Value) -> Self {
        let n = |k: &str| usage.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
        Usage {
            input: n("input_tokens"),
            cache_creation: n("cache_creation_input_tokens"),
            cache_read: n("cache_read_input_tokens"),
            output: n("output_tokens"),
        }
    }
}

fn extract_user_text(record: &Value) -> Option<String> {
    let content = record.get("message")?.get("content")?;
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        for item in arr {
            if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    let t = t.trim();
                    if !t.is_empty() {
                        return Some(t.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn decode_path_leading_hyphen() {
        assert_eq!(
            decode_project_path("-Users-michael-dev"),
            "/Users/michael/dev"
        );
    }

    #[test]
    fn decode_path_no_leading_hyphen() {
        assert_eq!(decode_project_path("home-user"), "home/user");
    }

    #[test]
    fn load_session_extracts_title_and_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"user","timestamp":"2026-06-01T10:00:00Z","message":{{"role":"user","content":[{{"type":"text","text":"implement auth feature"}}]}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","timestamp":"2026-06-01T10:01:00Z","message":{{"role":"assistant","content":[],"usage":{{"input_tokens":100,"cache_creation_input_tokens":500,"cache_read_input_tokens":2000,"output_tokens":300}}}}}}"#).unwrap();

        let record = load_session(&path, "/test/project").unwrap().unwrap();
        assert_eq!(record.title, "implement auth feature");
        assert!(matches!(
            record.effort,
            meridian_core::record::EffortUnits::Tokens {
                input: 600,
                output: 300
            }
        ));
        assert_eq!(record.metadata.get("project").unwrap(), "/test/project");
    }

    #[test]
    fn load_session_uses_ai_title() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"user","timestamp":"2026-06-01T10:00:00Z","message":{{"role":"user","content":[{{"type":"text","text":"implement auth"}}]}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"ai-title","title":"Authentication System Implementation"}}"#
        )
        .unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50}}}}}}"#).unwrap();

        let record = load_session(&path, "/test").unwrap().unwrap();
        assert_eq!(record.title, "Authentication System Implementation");
    }

    #[test]
    fn load_session_empty_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.jsonl");
        std::fs::write(&path, r#"{"type":"mode"}"#).unwrap();
        let result = load_session(&path, "/test").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_session_sums_multiple_assistant_turns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("multi.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"user","timestamp":"2026-06-01T10:00:00Z","message":{{"role":"user","content":[{{"type":"text","text":"hello"}}]}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":200}}}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":50,"cache_creation_input_tokens":200,"cache_read_input_tokens":0,"output_tokens":100}}}}}}"#).unwrap();

        let record = load_session(&path, "/test").unwrap().unwrap();
        assert!(matches!(
            record.effort,
            meridian_core::record::EffortUnits::Tokens {
                input: 350,
                output: 300
            }
        ));
    }

    fn write_session(lines: &[&str]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"user","timestamp":"2026-06-01T10:00:00Z","message":{{"role":"user","content":[{{"type":"text","text":"hi"}}]}}}}"#).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        (dir, path)
    }

    fn tokens(r: &UsageRecord) -> (u64, u64) {
        match r.effort {
            EffortUnits::Tokens { input, output } => (input, output),
            _ => panic!("expected token effort"),
        }
    }

    /// Streamed rows repeat one `message.id` with growing usage; only the last counts.
    #[test]
    fn repeated_message_id_is_counted_once_last_row_wins() {
        let (_d, path) = write_session(&[
            r#"{"type":"assistant","message":{"id":"msg_1","usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":5}}}"#,
            r#"{"type":"assistant","message":{"id":"msg_1","usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":40}}}"#,
            r#"{"type":"assistant","message":{"id":"msg_2","usage":{"input_tokens":3,"cache_creation_input_tokens":0,"cache_read_input_tokens":2000,"output_tokens":7}}}"#,
        ]);
        let r = load_session(&path, "/p").unwrap().unwrap();
        // msg_1 once (10+100 in, 40 out) plus msg_2 (3 in, 7 out).
        assert_eq!(tokens(&r), (113, 47));
        assert_eq!(r.metadata.get("cache_read_tokens").unwrap(), "3000");
    }

    /// `<synthetic>` rows are client-inserted and never reached the API.
    #[test]
    fn synthetic_rows_are_skipped() {
        let (_d, path) = write_session(&[
            r#"{"type":"assistant","message":{"id":"msg_s","model":"<synthetic>","usage":{"input_tokens":900,"cache_creation_input_tokens":0,"cache_read_input_tokens":900,"output_tokens":900}}}"#,
            r#"{"type":"assistant","message":{"id":"msg_r","model":"claude-sonnet-5-5","usage":{"input_tokens":1,"cache_creation_input_tokens":2,"cache_read_input_tokens":3,"output_tokens":4}}}"#,
        ]);
        let r = load_session(&path, "/p").unwrap().unwrap();
        assert_eq!(tokens(&r), (3, 4));
        assert_eq!(r.metadata.get("cache_read_tokens").unwrap(), "3");
    }
}
