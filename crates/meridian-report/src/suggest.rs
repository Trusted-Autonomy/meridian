use anyhow::Result;
use std::collections::HashMap;

use crate::summary::CategorySummary;

pub struct SuggestConfig {
    pub threshold: f32,
    pub sample_size: usize,
    pub model: String,
    pub api_key: String,
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self {
            threshold: 0.25,
            sample_size: 5,
            model: "claude-haiku-4-5-20251001".to_string(),
            api_key: String::new(),
        }
    }
}

/// Call Claude API to generate realignment suggestions for a low-scoring category×KPI pair.
pub fn generate_suggestion(
    config: &SuggestConfig,
    category_label: &str,
    kpi_label: &str,
    kpi_description: &str,
    current_score: f32,
    sample_titles: &[String],
) -> Result<Vec<String>> {
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

    let prompt = format!(
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
        .header("x-api-key", &config.api_key)
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

    let suggestions = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ')
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect();

    Ok(suggestions)
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
}
