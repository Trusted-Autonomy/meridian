use anyhow::{bail, Result};
use clap::Args;
use meridian_config::MeridianConfig;
use meridian_core::record::UsageRecord;
use meridian_core::scorer::KeywordScorer;
use meridian_ingest::{generic, ta::TaSource};
use meridian_report::{suggest as suggest_lib, summary};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct SuggestArgs {
    /// Source type: ta, jsonl (default: auto-detect)
    #[arg(long)]
    pub source: Option<String>,

    /// Path to data file or TA project root
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// KPI alignment threshold — categories below this get suggestions (overrides config)
    #[arg(long)]
    pub threshold: Option<f32>,

    /// Show which pairs would get suggestions without calling the API
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: SuggestArgs, config_path: &Path) -> Result<()> {
    let config = MeridianConfig::load_or_default(config_path);
    let taxonomy = config.taxonomy();

    if taxonomy.kpis.is_empty() {
        bail!(
            "No KPIs defined. Add [[kpis]] entries to meridian.toml first.\n\
             Run `meridian init` to create a starter config."
        );
    }

    let scorer = KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis);
    let records = load_records(&args, &config)?;

    if records.is_empty() {
        println!("No records found.");
        return Ok(());
    }

    let classified = scorer.classify_batch(&records);
    let report = summary::summarise(&classified);

    let threshold = args.threshold.unwrap_or(config.suggest.threshold);

    let kpi_labels: HashMap<String, String> = taxonomy
        .kpis
        .iter()
        .map(|k| (k.id.clone(), k.label.clone()))
        .collect();
    let kpi_descs: HashMap<String, String> = taxonomy
        .kpis
        .iter()
        .map(|k| (k.id.clone(), k.description.clone()))
        .collect();
    let cat_labels: HashMap<String, String> = taxonomy
        .categories
        .iter()
        .map(|c| (c.id.clone(), c.label.clone()))
        .collect();

    let low_pairs =
        suggest_lib::find_low_alignment_pairs(&report.by_category, threshold, &kpi_labels);

    if low_pairs.is_empty() {
        println!(
            "All category×KPI pairs are above the {:.0}% alignment threshold. No suggestions needed.",
            threshold * 100.0
        );
        return Ok(());
    }

    println!(
        "\nLow KPI Alignment ({} pair(s) below {:.0}% threshold):",
        low_pairs.len(),
        threshold * 100.0
    );
    for (cat_id, kpi_label, score) in &low_pairs {
        let cat_label = cat_labels
            .get(cat_id)
            .cloned()
            .unwrap_or_else(|| cat_id.clone());
        println!("  {} x {} — {:.0}%", cat_label, kpi_label, score * 100.0);
    }

    if args.dry_run {
        println!("\n(dry run — run without --dry-run to generate suggestions via Claude API)");
        return Ok(());
    }

    let api_key = config
        .suggest
        .api_key
        .clone()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Anthropic API key required for suggestions.\n\
                 Set ANTHROPIC_API_KEY env var or add api_key to [suggest] in meridian.toml."
            )
        })?;

    let suggest_config = suggest_lib::SuggestConfig {
        threshold,
        sample_size: config.suggest.sample_size,
        model: config.suggest.model.clone(),
        api_key,
    };

    println!();
    for (cat_id, kpi_label, score) in &low_pairs {
        let cat_label = cat_labels
            .get(cat_id)
            .cloned()
            .unwrap_or_else(|| cat_id.clone());

        let samples: Vec<String> = classified
            .iter()
            .filter(|r| &r.category_id == cat_id)
            .take(suggest_config.sample_size)
            .map(|r| r.record.title.clone())
            .collect();

        // kpi_label here is the human label; look up description by matching label back to id
        let kpi_desc = kpi_labels
            .iter()
            .find(|(_, v)| v.as_str() == kpi_label.as_str())
            .and_then(|(id, _)| kpi_descs.get(id))
            .cloned()
            .unwrap_or_default();

        println!(
            "-- {} x {} ({:.0}% alignment) --",
            cat_label,
            kpi_label,
            score * 100.0
        );
        match suggest_lib::generate_suggestion(
            &suggest_config,
            &cat_label,
            kpi_label,
            &kpi_desc,
            *score,
            &samples,
        ) {
            Ok(suggestions) => {
                for (i, s) in suggestions.iter().enumerate() {
                    println!("  {}. {}", i + 1, s);
                }
            }
            Err(e) => eprintln!("  Error generating suggestion: {e}"),
        }
        println!();
    }

    Ok(())
}

fn load_records(args: &SuggestArgs, config: &MeridianConfig) -> Result<Vec<UsageRecord>> {
    let source_type = args.source.as_deref().unwrap_or("auto");
    match source_type {
        "ta" => {
            let root = args
                .path
                .as_deref()
                .or_else(|| config.source.ta_project_root.as_deref().map(Path::new))
                .unwrap_or(Path::new("."));
            let vel_path = root.join(".ta").join("velocity-history.jsonl");
            if vel_path.exists() {
                TaSource::load_velocity(&vel_path)
            } else {
                TaSource::discover(root)
                    .ok_or_else(|| anyhow::anyhow!("No .ta/velocity-history.jsonl found"))
                    .and_then(|p| TaSource::load_velocity(&p))
            }
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
        _ => {
            if let Some(vel) = TaSource::discover(Path::new(".")) {
                TaSource::load_velocity(&vel)
            } else if let Some(p) = config.source.jsonl.as_deref() {
                generic::load_jsonl(Path::new(p))
            } else {
                bail!("No data source found. Use --source ta or --source jsonl --path file.jsonl")
            }
        }
    }
}
