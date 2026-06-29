use anyhow::Result;
use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};

use crate::summary::CategorySummary;

pub struct SuggestConfig {
    pub threshold: f32,
    pub sample_size: usize,
    pub model: String,
    /// Explicit API key. When None and use_claude_cli is false, generate_suggestion returns Err.
    pub api_key: Option<String>,
    /// Route suggestions through `claude --print` instead of the HTTP API.
    pub use_claude_cli: bool,
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self {
            threshold: 0.25,
            sample_size: 5,
            model: "claude-haiku-4-5-20251001".to_string(),
            api_key: None,
            use_claude_cli: false,
        }
    }
}

/// Returns true if the `claude` binary is on PATH and responds to --version.
pub fn claude_cli_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn build_prompt(
    config: &SuggestConfig,
    category_label: &str,
    kpi_label: &str,
    kpi_description: &str,
    current_score: f32,
    sample_titles: &[String],
) -> String {
    let samples = if sample_titles.is_empty() {
        "(no examples available)".to_string()
    } else {
        sample_titles
            .iter()
            .take(config.sample_size)
            .enumerate()
            .map(|(i, t)| format!("  {}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "You are analyzing how an AI-assisted team's work aligns with company goals.\n\
         \n\
         Category: {category}\n\
         KPI: {kpi} — {kpi_desc}\n\
         Current alignment score: {score:.0}% (target: >{threshold:.0}%)\n\
         \n\
         Recent work titles in this category:\n{samples}\n\
         \n\
         Give 2-3 concrete, specific suggestions for how future work in the '{category}' category \
         could be structured or framed to better advance the '{kpi}' goal. \
         Be practical — suggest changes to how work is scoped, titled, or prioritized, \
         not just to add a KPI mention. \
         Format as a numbered list, one suggestion per line, no preamble.",
        category = category_label,
        kpi = kpi_label,
        kpi_desc = kpi_description,
        score = current_score * 100.0,
        threshold = config.threshold * 100.0,
        samples = samples,
    )
}

fn parse_suggestions(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ')
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect()
}

/// Generate suggestions by piping the prompt to `claude --print`.
/// No API key needed — uses the user's existing claude CLI login.
pub fn generate_suggestion_via_cli(
    config: &SuggestConfig,
    category_label: &str,
    kpi_label: &str,
    kpi_description: &str,
    current_score: f32,
    sample_titles: &[String],
) -> Result<Vec<String>> {
    let prompt = build_prompt(
        config,
        category_label,
        kpi_label,
        kpi_description,
        current_score,
        sample_titles,
    );

    let mut child = Command::new("claude")
        .arg("--print")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!("Failed to start claude CLI: {e}. Install from https://claude.ai/code")
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write to claude CLI stdin: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("claude CLI wait failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "claude CLI exited with status {}: {}",
            output.status,
            stderr.trim()
        );
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(parse_suggestions(&text))
}

/// Call Claude to generate realignment suggestions for a low-scoring category×KPI pair.
/// Uses the claude CLI if `use_claude_cli` is true, otherwise calls the Anthropic HTTP API.
pub fn generate_suggestion(
    config: &SuggestConfig,
    category_label: &str,
    kpi_label: &str,
    kpi_description: &str,
    current_score: f32,
    sample_titles: &[String],
) -> Result<Vec<String>> {
    if config.use_claude_cli {
        return generate_suggestion_via_cli(
            config,
            category_label,
            kpi_label,
            kpi_description,
            current_score,
            sample_titles,
        );
    }

    let api_key = config.api_key.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "Set ANTHROPIC_API_KEY or install the claude CLI (https://claude.ai/code) to enable suggestions."
        )
    })?;

    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct Request {
        model: String,
        max_tokens: u32,
        messages: Vec<Message>,
    }

    #[derive(Deserialize)]
    struct Response {
        content: Vec<ContentBlock>,
    }

    #[derive(Deserialize)]
    struct ContentBlock {
        text: String,
    }

    let prompt = build_prompt(
        config,
        category_label,
        kpi_label,
        kpi_description,
        current_score,
        sample_titles,
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let req = Request {
        model: config.model.clone(),
        max_tokens: 512,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
    };

    let resp: Response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&req)
        .send()
        .map_err(|e| anyhow::anyhow!("Anthropic API request failed: {e}"))?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("Anthropic API error: {e}"))?
        .json()
        .map_err(|e| anyhow::anyhow!("Anthropic API response parse error: {e}"))?;

    let text = resp
        .content
        .into_iter()
        .map(|c| c.text)
        .collect::<Vec<_>>()
        .join("");

    Ok(parse_suggestions(&text))
}

/// Heuristically strip context injected by TA/CLAUDE.md from a raw prompt.
/// Keeps only likely user-authored content for title extraction.
pub fn strip_injected_context(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut skip = false;
    for line in text.lines() {
        // Skip CLAUDE.md injection block
        if line
            .trim_start()
            .starts_with("# Claude Code Project Instructions")
        {
            skip = true;
        }
        // Drop HTML comment lines (<!-- status: ... --> etc.)
        if line.trim_start().starts_with("<!--") {
            continue;
        }
        // Stop skipping on a non-heading non-empty line
        if skip && !line.starts_with('#') && !line.trim().is_empty() {
            skip = false;
        }
        if !skip {
            lines.push(line);
        }
    }
    let joined = lines.join("\n");
    if joined.len() > 3000 {
        joined[..3000].to_string()
    } else {
        joined
    }
}

/// Extract a concise work title (≤8 words) from raw prompt text.
/// Uses the claude CLI if available, otherwise the Anthropic HTTP API via `config`.
pub fn summarize_title(config: &SuggestConfig, raw_text: &str) -> Result<String> {
    let stripped = strip_injected_context(raw_text);
    if stripped.trim().is_empty() {
        anyhow::bail!("No content found after stripping injected context");
    }

    let prompt = format!(
        "Extract the user's actual work request from the text below as a concise title \
         (8 words max, no quotes, no punctuation at end).\n\
         Respond with ONLY the title — no explanation, no preamble.\n\n\
         Text:\n{text}",
        text = stripped.trim()
    );

    let result = if config.use_claude_cli || (config.api_key.is_none() && claude_cli_available()) {
        let mut child = std::process::Command::new("claude")
            .arg("--print")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start claude CLI: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to write to claude CLI stdin: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| anyhow::anyhow!("claude CLI wait failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "claude CLI exited with status {}: {}",
                output.status,
                stderr.trim()
            );
        }

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        let api_key = config.api_key.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "Set ANTHROPIC_API_KEY or install the claude CLI to enable title summarization."
            )
        })?;

        use serde::{Deserialize, Serialize};
        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }
        #[derive(Serialize)]
        struct Request {
            model: String,
            max_tokens: u32,
            messages: Vec<Message>,
        }
        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
        }
        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let req = Request {
            model: config.model.clone(),
            max_tokens: 32,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let resp: Response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .map_err(|e| anyhow::anyhow!("Anthropic API request failed: {e}"))?
            .error_for_status()
            .map_err(|e| anyhow::anyhow!("Anthropic API error: {e}"))?
            .json()
            .map_err(|e| anyhow::anyhow!("Anthropic API response parse error: {e}"))?;

        resp.content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    };

    let words: Vec<&str> = result.split_whitespace().collect();
    Ok(words.into_iter().take(8).collect::<Vec<_>>().join(" "))
}

/// Return (category_id, kpi_label, score) pairs below threshold, sorted by score ascending.
pub fn find_low_alignment_pairs(
    summary: &[CategorySummary],
    threshold: f32,
    kpi_labels: &HashMap<String, String>,
) -> Vec<(String, String, f32)> {
    let mut pairs = Vec::new();
    for cat in summary {
        for (kpi_id, &score) in &cat.kpi_alignment {
            if score < threshold {
                let kpi_label = kpi_labels
                    .get(kpi_id)
                    .cloned()
                    .unwrap_or_else(|| kpi_id.clone());
                pairs.push((cat.category_id.clone(), kpi_label, score));
            }
        }
    }
    pairs.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(cat_id: &str, kpi_scores: &[(&str, f32)]) -> CategorySummary {
        CategorySummary {
            category_id: cat_id.to_string(),
            record_count: 1,
            total_effort: 100.0,
            effort_pct: 100.0,
            kpi_alignment: kpi_scores
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
        }
    }

    #[test]
    fn find_low_pairs_below_threshold() {
        let summaries = vec![
            make_summary("code", &[("revenue", 0.1), ("eng_vel", 0.8)]),
            make_summary("docs", &[("revenue", 0.5), ("eng_vel", 0.6)]),
        ];
        let labels: HashMap<String, String> = [
            ("revenue".to_string(), "Revenue Growth".to_string()),
            ("eng_vel".to_string(), "Engineering Velocity".to_string()),
        ]
        .into();
        let pairs = find_low_alignment_pairs(&summaries, 0.3, &labels);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "code");
        assert_eq!(pairs[0].1, "Revenue Growth");
        assert!((pairs[0].2 - 0.1).abs() < 1e-5);
    }

    #[test]
    fn find_low_pairs_sorted_ascending() {
        let summaries = vec![make_summary(
            "code",
            &[("kpi_a", 0.2), ("kpi_b", 0.05), ("kpi_c", 0.15)],
        )];
        let labels: HashMap<String, String> = Default::default();
        let pairs = find_low_alignment_pairs(&summaries, 0.3, &labels);
        assert_eq!(pairs.len(), 3);
        assert!(pairs[0].2 <= pairs[1].2);
        assert!(pairs[1].2 <= pairs[2].2);
    }

    #[test]
    fn suggest_via_cli_skips_when_no_claude() {
        // If the claude binary is not on PATH, skip rather than fail.
        if !claude_cli_available() {
            return;
        }
        let config = SuggestConfig {
            use_claude_cli: true,
            ..Default::default()
        };
        let result = generate_suggestion_via_cli(
            &config,
            "Code Implementation",
            "Engineering Velocity",
            "Ship quality features faster",
            0.1,
            &["implement auth".to_string(), "fix login bug".to_string()],
        );
        assert!(result.is_ok(), "CLI suggestion failed: {:?}", result.err());
        assert!(
            !result.unwrap().is_empty(),
            "CLI suggestion returned no lines"
        );
    }
}
