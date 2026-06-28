use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use clap::Args;
use meridian_config::MeridianConfig;
use meridian_core::record::UsageRecord;
use meridian_core::scorer::KeywordScorer;
use meridian_ingest::{claude_code, generic, ta::TaSource};
use meridian_report::panel::{PanelResult, PanelScorer};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DISSENT_THRESHOLD: f32 = 0.20;

#[derive(Args)]
pub struct ReportArgs {
    /// Time window: e.g. "7d", "30d", "2w", "2026-06-01" (default: 7d)
    #[arg(long, default_value = "7d")]
    pub since: String,

    /// Output format: table, json, csv (overrides config)
    #[arg(long)]
    pub format: Option<String>,

    /// Source type: ta, jsonl, claude-code (default: auto-detect)
    #[arg(long)]
    pub source: Option<String>,

    /// Path to data file or TA project root (overrides config)
    #[arg(long)]
    pub path: Option<PathBuf>,
}

pub fn parse_since(s: &str) -> Result<DateTime<Utc>> {
    if let Some(days_str) = s.strip_suffix('d') {
        let days: i64 = days_str
            .parse()
            .with_context(|| format!("invalid days value in '{s}'"))?;
        Ok(Utc::now() - chrono::Duration::days(days))
    } else if let Some(weeks_str) = s.strip_suffix('w') {
        let weeks: i64 = weeks_str
            .parse()
            .with_context(|| format!("invalid weeks value in '{s}'"))?;
        Ok(Utc::now() - chrono::Duration::weeks(weeks))
    } else {
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .with_context(|| format!("invalid date '{s}' — use YYYY-MM-DD, Nd, or Nw"))?;
        Ok(date.and_hms_opt(0, 0, 0).expect("valid hms").and_utc())
    }
}

fn load_records(args: &ReportArgs, config: &MeridianConfig) -> Result<Vec<UsageRecord>> {
    let source_type: &str = args
        .source
        .as_deref()
        .or_else(|| {
            if config.source.ta_project_root.is_some() {
                Some("ta")
            } else if config.source.claude_code_dir.is_some() {
                Some("claude-code")
            } else if config.source.jsonl.is_some() {
                Some("jsonl")
            } else {
                None
            }
        })
        .unwrap_or("auto");

    match source_type {
        "ta" => {
            let root = args
                .path
                .as_deref()
                .or_else(|| config.source.ta_project_root.as_deref().map(Path::new))
                .unwrap_or(Path::new("."));
            let vel_path = {
                let direct = root.join(".ta").join("velocity-history.jsonl");
                if direct.exists() {
                    direct
                } else {
                    TaSource::discover(root).ok_or_else(|| {
                        anyhow::anyhow!(
                            "No .ta/velocity-history.jsonl found under {}.",
                            root.display()
                        )
                    })?
                }
            };
            TaSource::load_velocity(&vel_path)
        }
        "jsonl" => {
            let path = args
                .path
                .as_deref()
                .or_else(|| config.source.jsonl.as_deref().map(Path::new))
                .ok_or_else(|| {
                    anyhow::anyhow!("Specify --path or set source.jsonl in meridian.toml")
                })?;
            generic::load_jsonl(path)
        }
        "claude-code" => {
            let dir = args
                .path
                .clone()
                .or_else(|| config.source.claude_code_dir.as_deref().map(PathBuf::from))
                .unwrap_or_else(claude_code::default_projects_dir);
            claude_code::load_all(&dir)
        }
        "auto" => {
            if let Some(vel) = TaSource::discover(Path::new(".")) {
                TaSource::load_velocity(&vel)
            } else if let Some(p) = config.source.jsonl.as_deref() {
                generic::load_jsonl(Path::new(p))
            } else {
                let cc_dir = config
                    .source
                    .claude_code_dir
                    .as_deref()
                    .map(PathBuf::from)
                    .unwrap_or_else(claude_code::default_projects_dir);
                if cc_dir.exists() {
                    claude_code::load_all(&cc_dir)
                } else {
                    bail!(
                        "No data source found. Options:\n\
                         \n  meridian report --source ta\
                         \n  meridian report --source claude-code\
                         \n  meridian report --source jsonl --path records.jsonl"
                    )
                }
            }
        }
        other => bail!("Unknown source '{other}'. Valid: ta, jsonl, claude-code"),
    }
}

pub fn run(args: ReportArgs, config_path: &Path) -> Result<()> {
    let config = MeridianConfig::load_or_default(config_path);
    let since = parse_since(&args.since)?;
    let panel = config.panel_effective();

    let all_records = load_records(&args, &config)?;
    let records: Vec<UsageRecord> = all_records
        .into_iter()
        .filter(|r| r.timestamp >= since)
        .collect();

    if records.is_empty() {
        println!("No records found in the '{}' window.", args.since);
        return Ok(());
    }

    let scorer = PanelScorer::new(&panel);
    let results = scorer.score_batch(&records);

    // Also compute top KPI per record if KPIs are configured.
    let taxonomy = config.taxonomy();
    let kpi_scorer = if taxonomy.kpis.is_empty() {
        None
    } else {
        Some(KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis))
    };
    let kpi_labels: HashMap<String, String> = taxonomy
        .kpis
        .iter()
        .map(|k| (k.id.clone(), k.label.clone()))
        .collect();

    let format = args.format.as_deref().unwrap_or(&config.report.format);
    match format {
        "json" => print_json(&results),
        "csv" => print_csv(
            &results,
            &panel.iter().map(|m| m.role.clone()).collect::<Vec<_>>(),
            &kpi_scorer,
            &kpi_labels,
        ),
        _ => print_table(
            &results,
            &since,
            &args.since,
            &panel.iter().map(|m| m.role.clone()).collect::<Vec<_>>(),
            &kpi_scorer,
            &kpi_labels,
        ),
    }

    Ok(())
}

fn print_json(results: &[PanelResult]) {
    println!(
        "{}",
        serde_json::to_string_pretty(results).unwrap_or_default()
    );
}

fn print_csv(
    results: &[PanelResult],
    roles: &[String],
    kpi_scorer: &Option<KeywordScorer>,
    kpi_labels: &HashMap<String, String>,
) {
    // Header
    let mut header = "timestamp,title,source".to_string();
    for role in roles {
        header.push(',');
        header.push_str(role);
    }
    header.push_str(",consensus,dissent");
    if kpi_scorer.is_some() {
        for kpi_id in kpi_labels.keys() {
            header.push(',');
            header.push_str(kpi_id);
        }
    }
    println!("{header}");

    for result in results {
        let ts = result.record.timestamp.format("%Y-%m-%dT%H:%M:%SZ");
        let title = result.record.title.replace(',', ";");
        let mut row = format!("{ts},{title},{}", result.record.source);

        for role in roles {
            let score = result
                .scores
                .iter()
                .find(|s| &s.role == role)
                .map(|s| s.score)
                .unwrap_or(0.0);
            row.push_str(&format!(",{score:.3}"));
        }
        row.push_str(&format!(",{:.3},{:.3}", result.consensus, result.dissent));

        if let Some(ks) = kpi_scorer {
            let classified = ks.classify(&result.record);
            for kpi_id in kpi_labels.keys() {
                let score = classified
                    .kpi_scores
                    .iter()
                    .find(|s| &s.kpi_id == kpi_id)
                    .map(|s| s.score)
                    .unwrap_or(0.0);
                row.push_str(&format!(",{score:.3}"));
            }
        }
        println!("{row}");
    }
}

fn print_table(
    results: &[PanelResult],
    since: &DateTime<Utc>,
    since_str: &str,
    roles: &[String],
    kpi_scorer: &Option<KeywordScorer>,
    kpi_labels: &HashMap<String, String>,
) {
    let now = Utc::now();
    let from_str = since.format("%Y-%m-%d").to_string();
    let to_str = now.format("%Y-%m-%d").to_string();
    let n = results.len();

    // Abbreviated role labels for column headers.
    let short_labels: Vec<String> = roles
        .iter()
        .map(|r| match r.as_str() {
            "ceo" => "CEO".to_string(),
            "cto" => "CTO".to_string(),
            "head_of_product" => "PROD".to_string(),
            "head_of_engineering" => "ENG".to_string(),
            other => {
                let s: String = other
                    .split('_')
                    .map(|w| w.chars().next().unwrap_or(' ').to_uppercase().to_string())
                    .collect::<Vec<_>>()
                    .join("");
                s
            }
        })
        .collect();

    println!();
    println!(
        "Report  {} → {}   ({} session{})",
        from_str,
        to_str,
        n,
        if n == 1 { "" } else { "s" }
    );
    println!();

    // Column widths
    let title_w = results
        .iter()
        .map(|r| r.record.title.len().min(50))
        .max()
        .unwrap_or(20)
        .max(7); // "Session"

    // Header line
    let role_header: String = short_labels
        .iter()
        .map(|l| format!("{:>5}", l))
        .collect::<Vec<_>>()
        .join(" ");

    let has_kpis = kpi_scorer.is_some() && !kpi_labels.is_empty();
    let kpi_col = if has_kpis { "  Top KPI" } else { "" };

    println!(
        " {:<title_w$}  {}  {:>9}  {:>7}{}",
        "Session", role_header, "Consensus", "Dissent", kpi_col
    );

    let rule_len = title_w + 2 + role_header.len() + 2 + 9 + 2 + 7 + if has_kpis { 10 } else { 0 };
    println!(" {}", "─".repeat(rule_len));

    // Collect per-role totals for period averages
    let mut role_totals: Vec<f32> = vec![0.0; roles.len()];
    let mut consensus_total = 0.0f32;
    let mut dissent_total = 0.0f32;
    let mut high_dissent: Vec<(&PanelResult, String, String)> = Vec::new();

    for result in results {
        let title_trunc: String = result.record.title.chars().take(title_w).collect();

        let role_scores: String = roles
            .iter()
            .enumerate()
            .map(|(i, role)| {
                let score = result
                    .scores
                    .iter()
                    .find(|s| &s.role == role)
                    .map(|s| s.score)
                    .unwrap_or(0.0);
                role_totals[i] += score;
                format!("{:>5.2}", score)
            })
            .collect::<Vec<_>>()
            .join(" ");

        consensus_total += result.consensus;
        dissent_total += result.dissent;

        let dissent_flag = if result.dissent > DISSENT_THRESHOLD {
            "!"
        } else {
            " "
        };

        let top_kpi_col = if has_kpis {
            let ks = kpi_scorer.as_ref().unwrap();
            let classified = ks.classify(&result.record);
            let top = classified.kpi_scores.iter().max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            top.map(|k| format!("  {}", k.kpi_id)).unwrap_or_default()
        } else {
            String::new()
        };

        println!(
            " {:<title_w$}  {}  {:>9.2}  {:>6.2}{}{}",
            title_trunc, role_scores, result.consensus, result.dissent, dissent_flag, top_kpi_col
        );

        if result.dissent > DISSENT_THRESHOLD {
            // Find champion and skeptic scores for the dissent message.
            let champ_score = result
                .scores
                .iter()
                .find(|s| s.role == result.champion)
                .map(|s| s.score)
                .unwrap_or(0.0);
            let skept_score = result
                .scores
                .iter()
                .find(|s| s.role == result.skeptic)
                .map(|s| s.score)
                .unwrap_or(0.0);
            high_dissent.push((
                result,
                format!("{} {:.2}", result.champion, champ_score),
                format!("{} {:.2}", result.skeptic, skept_score),
            ));
        }
    }

    let count = results.len() as f32;
    let avg_roles: String = role_totals
        .iter()
        .map(|t| format!("{:>5.2}", t / count))
        .collect::<Vec<_>>()
        .join(" ");

    println!(" {}", "─".repeat(rule_len));
    println!(
        " {:<title_w$}  {}  {:>9.2}  {:>7.2}",
        "Period average",
        avg_roles,
        consensus_total / count,
        dissent_total / count
    );

    if !high_dissent.is_empty() {
        println!();
        println!(" High dissent (>{DISSENT_THRESHOLD:.2}):");
        for (result, champ, skept) in &high_dissent {
            let title_short: String = result.record.title.chars().take(50).collect();
            println!("   \"{title_short}\" — {champ} vs {skept}");
        }
    }

    if results.is_empty() {
        println!("\n No records in '{since_str}' window.");
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::record::EffortUnits;

    #[test]
    fn parse_since_days() {
        let dt = parse_since("7d").unwrap();
        let diff = Utc::now() - dt;
        assert!(diff.num_hours() >= 6 * 24 && diff.num_hours() <= 8 * 24);
    }

    #[test]
    fn parse_since_weeks() {
        let dt = parse_since("2w").unwrap();
        let diff = Utc::now() - dt;
        assert!(diff.num_days() >= 13 && diff.num_days() <= 15);
    }

    #[test]
    fn parse_since_date() {
        let dt = parse_since("2026-01-01").unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-01-01");
    }

    #[test]
    fn parse_since_invalid() {
        assert!(parse_since("notadate").is_err());
    }

    #[test]
    fn report_filters_by_since() {
        use meridian_core::panel::default_panel;

        let now = Utc::now();
        let recent = UsageRecord {
            id: "r1".into(),
            timestamp: now - chrono::Duration::days(1),
            title: "recent work".into(),
            effort: EffortUnits::Unknown,
            source: "test".into(),
            phase: None,
            metadata: Default::default(),
        };
        let old = UsageRecord {
            id: "r2".into(),
            timestamp: now - chrono::Duration::days(8),
            title: "old work".into(),
            effort: EffortUnits::Unknown,
            source: "test".into(),
            phase: None,
            metadata: Default::default(),
        };

        let since = parse_since("7d").unwrap();
        let records: Vec<UsageRecord> = vec![recent, old]
            .into_iter()
            .filter(|r| r.timestamp >= since)
            .collect();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "r1");

        // Verify panel scoring works on the filtered set.
        let panel = default_panel();
        let scorer = PanelScorer::new(&panel);
        let results = scorer.score_batch(&records);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].scores.len(), 4);
    }
}
