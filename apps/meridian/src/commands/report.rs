use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use clap::Args;
use meridian_config::MeridianConfig;
use meridian_core::record::{EffortUnits, UsageRecord};
use meridian_core::result::KpiScore;
use meridian_core::scorer::KeywordScorer;
use meridian_core::taxonomy::Kpi;
use meridian_ingest::{claude_code, generic, ta::TaSource};
use meridian_report::panel::{PanelResult, PanelScorer};
use serde_json;
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

    let taxonomy = config.taxonomy();
    let kpi_scorer = if taxonomy.kpis.is_empty() {
        None
    } else {
        Some(KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis))
    };

    let format = args.format.as_deref().unwrap_or(&config.report.format);
    match format {
        "json" => print_json(&results),
        "csv" => print_csv(
            &results,
            &panel.iter().map(|m| m.role.clone()).collect::<Vec<_>>(),
            &kpi_scorer,
            &taxonomy.kpis,
        ),
        _ => print_table(
            &results,
            &since,
            &args.since,
            &panel.iter().map(|m| m.role.clone()).collect::<Vec<_>>(),
            &kpi_scorer,
            &taxonomy.kpis,
        ),
    }

    Ok(())
}

fn format_effort(effort: &EffortUnits) -> String {
    match effort {
        EffortUnits::Tokens { input, output } => {
            let total = input + output;
            if total >= 1_000_000 {
                format!("{:.1}M", total as f64 / 1_000_000.0)
            } else if total >= 1_000 {
                format!("{}K", total / 1_000)
            } else {
                format!("{total}")
            }
        }
        EffortUnits::Seconds(s) => {
            if *s >= 3600 {
                format!("{:.1}h", *s as f64 / 3600.0)
            } else if *s >= 60 {
                format!("{}m", s / 60)
            } else {
                format!("{s}s")
            }
        }
        EffortUnits::Unknown => "-".to_string(),
    }
}

/// Weighted mean of per-KPI scores, using KPI.weight from config.
fn kpi_weighted_align(kpi_scores: &[KpiScore], kpis: &[Kpi]) -> f32 {
    if kpis.is_empty() || kpi_scores.is_empty() {
        return 0.0;
    }
    let total_weight: f32 = kpis.iter().map(|k| k.weight).sum();
    if total_weight == 0.0 {
        return 0.0;
    }
    let weighted_sum: f32 = kpi_scores
        .iter()
        .filter_map(|ks| {
            kpis.iter()
                .find(|k| k.id == ks.kpi_id)
                .map(|k| ks.score * k.weight)
        })
        .sum();
    weighted_sum / total_weight
}

/// Convert a role ID to a human-readable display string.
fn display_role(role: &str) -> String {
    match role {
        "ceo" => "CEO".to_string(),
        "cto" => "CTO".to_string(),
        "cfo" => "CFO".to_string(),
        _ => role
            .split('_')
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// First word of a KPI label, used as a compact identifier in the inline breakdown row.
fn kpi_short_label<'a>(kpi_id: &'a str, kpis: &'a [Kpi]) -> &'a str {
    kpis.iter()
        .find(|k| k.id == kpi_id)
        .and_then(|k| k.label.split_whitespace().next())
        .unwrap_or(kpi_id)
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
    kpis: &[Kpi],
) {
    let mut header = "timestamp,title,source,effort".to_string();
    for role in roles {
        header.push(',');
        header.push_str(role);
    }
    header.push_str(",consensus,dissent");
    if kpi_scorer.is_some() {
        for kpi in kpis {
            header.push(',');
            header.push_str(&kpi.id);
        }
        header.push_str(",kpi_align");
    }
    println!("{header}");

    for result in results {
        let ts = result.record.timestamp.format("%Y-%m-%dT%H:%M:%SZ");
        let title = result.record.title.replace(',', ";");
        let effort = format_effort(&result.record.effort);
        let mut row = format!("{ts},{title},{},{effort}", result.record.source);

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
            for kpi in kpis {
                let score = classified
                    .kpi_scores
                    .iter()
                    .find(|s| s.kpi_id == kpi.id)
                    .map(|s| s.score)
                    .unwrap_or(0.0);
                row.push_str(&format!(",{score:.3}"));
            }
            let align = kpi_weighted_align(&classified.kpi_scores, kpis);
            row.push_str(&format!(",{align:.3}"));
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
    kpis: &[Kpi],
) {
    let now = Utc::now();
    let from_str = since.format("%Y-%m-%d").to_string();
    let to_str = now.format("%Y-%m-%d").to_string();
    let n = results.len();
    let has_kpis = kpi_scorer.is_some() && !kpis.is_empty();

    println!();
    println!(
        " Report  {} -> {}   ({} session{})",
        from_str,
        to_str,
        n,
        if n == 1 { "" } else { "s" }
    );
    println!();

    let title_w = results
        .iter()
        .map(|r| r.record.title.len().min(52))
        .max()
        .unwrap_or(20)
        .max(7);

    // Session table header
    if has_kpis {
        println!(
            " {:<title_w$}  {:>6}  {:>9}  {:>9}  !",
            "Session", "Effort", "Consensus", "KPI-Align"
        );
    } else {
        println!(
            " {:<title_w$}  {:>6}  {:>9}  {:>7}",
            "Session", "Effort", "Consensus", "Dissent"
        );
    }
    let rule_len = title_w + 2 + 6 + 2 + 9 + 2 + 9 + 3;
    println!(" {}", "\u{2500}".repeat(rule_len));

    // Accumulators for averages
    let mut role_totals: Vec<f32> = vec![0.0; roles.len()];
    let mut consensus_total = 0.0f32;
    let mut dissent_total = 0.0f32;
    let mut kpi_align_total = 0.0f32;
    let mut high_dissent: Vec<(&PanelResult, String, String)> = Vec::new();

    for result in results {
        let title_trunc: String = result.record.title.chars().take(title_w).collect();
        let effort_str = format_effort(&result.record.effort);

        for (i, role) in roles.iter().enumerate() {
            let score = result
                .scores
                .iter()
                .find(|s| &s.role == role)
                .map(|s| s.score)
                .unwrap_or(0.0);
            role_totals[i] += score;
        }
        consensus_total += result.consensus;
        dissent_total += result.dissent;

        let dissent_flag = if result.dissent > DISSENT_THRESHOLD {
            "!"
        } else {
            " "
        };

        if has_kpis {
            let ks = kpi_scorer.as_ref().unwrap();
            let classified = ks.classify(&result.record);
            let align = kpi_weighted_align(&classified.kpi_scores, kpis);
            kpi_align_total += align;

            println!(
                " {:<title_w$}  {:>6}  {:>9.2}  {:>9.2}  {}",
                title_trunc, effort_str, result.consensus, align, dissent_flag
            );

            // Inline per-KPI breakdown row, sorted by score descending.
            let mut scores_sorted = classified.kpi_scores.clone();
            scores_sorted.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let kpi_line: String = scores_sorted
                .iter()
                .map(|ks| format!("{}: {:.2}", kpi_short_label(&ks.kpi_id, kpis), ks.score))
                .collect::<Vec<_>>()
                .join("  ");
            println!("    \u{25B8} {}", kpi_line);
        } else {
            println!(
                " {:<title_w$}  {:>6}  {:>9.2}  {:>7.2}",
                title_trunc, effort_str, result.consensus, result.dissent
            );
        }

        if result.dissent > DISSENT_THRESHOLD {
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
                format!("{} {:.2}", display_role(&result.champion), champ_score),
                format!("{} {:.2}", display_role(&result.skeptic), skept_score),
            ));
        }
    }

    let count = results.len() as f32;
    println!(" {}", "\u{2500}".repeat(rule_len));
    if has_kpis {
        println!(
            " {:<title_w$}  {:>6}  {:>9.2}  {:>9.2}",
            "Period average",
            "-",
            consensus_total / count,
            kpi_align_total / count
        );
    } else {
        println!(
            " {:<title_w$}  {:>6}  {:>9.2}  {:>7.2}",
            "Period average",
            "-",
            consensus_total / count,
            dissent_total / count
        );
    }

    // Panel averages section
    if !roles.is_empty() {
        println!();
        println!(" PANEL AVERAGES");
        let role_w = roles
            .iter()
            .map(|r| display_role(r).len())
            .max()
            .unwrap_or(10);
        for (i, role) in roles.iter().enumerate() {
            println!(
                "   {:<role_w$}  {:.2}",
                display_role(role),
                role_totals[i] / count
            );
        }
        println!(
            "   {:<role_w$}  {:.2}   Dissent {:.2}",
            "Consensus",
            consensus_total / count,
            dissent_total / count
        );
    }

    if !high_dissent.is_empty() {
        println!();
        println!(" High dissent (>{DISSENT_THRESHOLD:.2}):");
        for (result, champ, skept) in &high_dissent {
            let title_short: String = result.record.title.chars().take(52).collect();
            println!("   \"{title_short}\"");
            println!("     champion: {champ}  skeptic: {skept}");
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

        let panel = default_panel();
        let scorer = PanelScorer::new(&panel);
        let results = scorer.score_batch(&records);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].scores.len(), 4);
    }

    #[test]
    fn format_effort_tokens() {
        assert_eq!(
            format_effort(&EffortUnits::Tokens {
                input: 300_000,
                output: 40_000
            }),
            "340K"
        );
        assert_eq!(
            format_effort(&EffortUnits::Tokens {
                input: 900_000,
                output: 100_000
            }),
            "1.0M"
        );
    }

    #[test]
    fn format_effort_seconds() {
        assert_eq!(format_effort(&EffortUnits::Seconds(90)), "1m");
        assert_eq!(format_effort(&EffortUnits::Seconds(7200)), "2.0h");
    }

    #[test]
    fn format_effort_unknown() {
        assert_eq!(format_effort(&EffortUnits::Unknown), "-");
    }

    #[test]
    fn kpi_weighted_align_equal_weights() {
        let kpis = vec![
            Kpi {
                id: "a".into(),
                label: "A".into(),
                description: "".into(),
                weight: 1.0,
                metrics: vec![],
            },
            Kpi {
                id: "b".into(),
                label: "B".into(),
                description: "".into(),
                weight: 1.0,
                metrics: vec![],
            },
        ];
        let scores = vec![
            KpiScore {
                kpi_id: "a".into(),
                score: 0.8,
            },
            KpiScore {
                kpi_id: "b".into(),
                score: 0.4,
            },
        ];
        let align = kpi_weighted_align(&scores, &kpis);
        assert!((align - 0.6).abs() < 1e-5, "expected 0.6, got {align}");
    }

    #[test]
    fn kpi_weighted_align_unequal_weights() {
        let kpis = vec![
            Kpi {
                id: "a".into(),
                label: "A".into(),
                description: "".into(),
                weight: 2.0,
                metrics: vec![],
            },
            Kpi {
                id: "b".into(),
                label: "B".into(),
                description: "".into(),
                weight: 1.0,
                metrics: vec![],
            },
        ];
        let scores = vec![
            KpiScore {
                kpi_id: "a".into(),
                score: 0.9,
            },
            KpiScore {
                kpi_id: "b".into(),
                score: 0.0,
            },
        ];
        // (0.9*2 + 0.0*1) / 3 = 0.6
        let align = kpi_weighted_align(&scores, &kpis);
        assert!((align - 0.6).abs() < 1e-5, "expected 0.6, got {align}");
    }

    #[test]
    fn kpi_weighted_align_empty() {
        let align = kpi_weighted_align(&[], &[]);
        assert_eq!(align, 0.0);
    }

    #[test]
    fn display_role_known() {
        assert_eq!(display_role("ceo"), "CEO");
        assert_eq!(display_role("cto"), "CTO");
        assert_eq!(display_role("cfo"), "CFO");
    }

    #[test]
    fn display_role_snake_case() {
        assert_eq!(display_role("head_of_product"), "Head Of Product");
        assert_eq!(display_role("tech_director"), "Tech Director");
    }
}
