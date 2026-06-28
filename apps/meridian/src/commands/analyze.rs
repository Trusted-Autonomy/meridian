use anyhow::{bail, Result};
use clap::Args;
use meridian_config::MeridianConfig;
use meridian_core::scorer::KeywordScorer;
use meridian_ingest::{generic, ta::TaSource};
use meridian_report::{export, summary, table};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct AnalyzeArgs {
    /// Source type: ta, jsonl (default: auto-detect)
    #[arg(long)]
    pub source: Option<String>,

    /// Path to data file or TA project root (overrides config)
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Output format: table, json, csv (overrides config)
    #[arg(long)]
    pub format: Option<String>,
}

pub fn run(args: AnalyzeArgs, config_path: &Path) -> Result<()> {
    let config = MeridianConfig::load_or_default(config_path);
    let taxonomy = config.taxonomy();
    let scorer = KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis);

    let source_type: &str = args
        .source
        .as_deref()
        .or_else(|| {
            if config.source.ta_project_root.is_some() {
                Some("ta")
            } else if config.source.jsonl.is_some() {
                Some("jsonl")
            } else {
                None
            }
        })
        .unwrap_or("auto");

    let records = match source_type {
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
                } else if root
                    .file_name()
                    .is_some_and(|n| n == "velocity-history.jsonl")
                {
                    root.to_path_buf()
                } else {
                    TaSource::discover(root).ok_or_else(|| {
                        anyhow::anyhow!(
                            "No .ta/velocity-history.jsonl found under {}.\n\
                             Pass --path /path/to/project or use --source jsonl.",
                            root.display()
                        )
                    })?
                }
            };
            TaSource::load_velocity(&vel_path)?
        }
        "jsonl" => {
            let path = args
                .path
                .as_deref()
                .or_else(|| config.source.jsonl.as_deref().map(Path::new))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Specify --path /path/to/records.jsonl or set source.jsonl in meridian.toml"
                    )
                })?;
            generic::load_jsonl(path)?
        }
        "auto" => {
            if let Some(vel) = TaSource::discover(Path::new(".")) {
                eprintln!("Auto-detected TA source: {}", vel.display());
                TaSource::load_velocity(&vel)?
            } else if let Some(p) = config.source.jsonl.as_deref() {
                generic::load_jsonl(Path::new(p))?
            } else {
                bail!(
                    "No data source found. Run from a TA project directory, or:\n\
                     \n  meridian analyze --source jsonl --path records.jsonl\n\
                     \n  Or set [source] in meridian.toml"
                );
            }
        }
        other => bail!("Unknown source '{}'. Valid values: ta, jsonl", other),
    };

    if records.is_empty() {
        println!("No records found.");
        return Ok(());
    }

    let classified = scorer.classify_batch(&records);
    let report = summary::summarise(&classified);

    let labels: HashMap<String, String> = taxonomy
        .categories
        .iter()
        .map(|c| (c.id.clone(), c.label.clone()))
        .collect();

    let format = args.format.as_deref().unwrap_or(&config.report.format);
    match format {
        "json" => println!("{}", export::to_json(&report)?),
        "csv" => print!("{}", export::to_csv(&classified)),
        _ => table::print_summary(&report, &labels),
    }

    Ok(())
}
