// Interactive setup wizard: detects source, picks domain profile, writes meridian.toml.
// Uses dialoguer for prompts.

use anyhow::Result;
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect, Select};
use meridian_ingest::{claude_code, ta::TaSource};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct SetupArgs {
    /// Directory to create meridian.toml in (default: current directory)
    #[arg(default_value = ".")]
    pub dir: PathBuf,
}

struct KpiTemplate {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    weight: f32,
}

struct DomainProfile {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    kpis: Vec<KpiTemplate>,
}

fn domain_profiles() -> Vec<DomainProfile> {
    vec![
        DomainProfile {
            id: "saas",
            label: "SaaS / Software Product",
            description: "Engineering teams shipping a software product",
            kpis: vec![
                KpiTemplate {
                    id: "eng_velocity",
                    label: "Engineering Velocity",
                    description: "Ship quality features faster than competitors",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "product_quality",
                    label: "Product Quality",
                    description: "Reduce defects, improve stability, increase user satisfaction",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "customer_growth",
                    label: "Customer Growth",
                    description: "Acquire and retain paying customers",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "operational_efficiency",
                    label: "Operational Efficiency",
                    description: "Reduce cost-to-serve, automate toil, improve reliability",
                    weight: 0.8,
                },
            ],
        },
        DomainProfile {
            id: "gamedev",
            label: "Game Development",
            description: "Game studio shipping playable experiences",
            kpis: vec![
                KpiTemplate {
                    id: "gameplay_quality",
                    label: "Gameplay Quality",
                    description: "Core gameplay mechanics are fun, balanced, and polished",
                    weight: 1.2,
                },
                KpiTemplate {
                    id: "release_velocity",
                    label: "Release Velocity",
                    description: "Ship content updates and patches on schedule",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "player_retention",
                    label: "Player Retention",
                    description: "Keep players engaged day-over-day and week-over-week",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "technical_performance",
                    label: "Technical Performance",
                    description: "Frame rate, load times, and platform stability targets",
                    weight: 0.9,
                },
            ],
        },
        DomainProfile {
            id: "fintech",
            label: "Fintech / Financial Services",
            description: "Teams building financial products or services",
            kpis: vec![
                KpiTemplate {
                    id: "regulatory_compliance",
                    label: "Regulatory Compliance",
                    description: "Meet all legal, audit, and regulatory requirements",
                    weight: 1.5,
                },
                KpiTemplate {
                    id: "transaction_reliability",
                    label: "Transaction Reliability",
                    description: "Zero-downtime transaction processing with full auditability",
                    weight: 1.3,
                },
                KpiTemplate {
                    id: "security_posture",
                    label: "Security Posture",
                    description: "Protect customer assets and data, minimize breach risk",
                    weight: 1.2,
                },
                KpiTemplate {
                    id: "product_growth",
                    label: "Product Growth",
                    description: "Grow AUM, transaction volume, or active users",
                    weight: 1.0,
                },
            ],
        },
        DomainProfile {
            id: "ecommerce",
            label: "E-commerce / Marketplace",
            description: "Teams running online storefronts or marketplaces",
            kpis: vec![
                KpiTemplate {
                    id: "conversion_rate",
                    label: "Conversion Rate",
                    description: "Improve checkout completion, reduce cart abandonment",
                    weight: 1.2,
                },
                KpiTemplate {
                    id: "catalog_quality",
                    label: "Catalog Quality",
                    description: "Accurate product data, fast search, strong recommendations",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "ops_efficiency",
                    label: "Operational Efficiency",
                    description: "Reduce fulfillment cost, improve inventory management",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "customer_experience",
                    label: "Customer Experience",
                    description: "Fast, reliable, personalized shopping experience",
                    weight: 1.0,
                },
            ],
        },
        DomainProfile {
            id: "devtools",
            label: "Developer Tools / Platform",
            description: "Teams building tools and platforms for developers",
            kpis: vec![
                KpiTemplate {
                    id: "developer_experience",
                    label: "Developer Experience (DX)",
                    description: "Time-to-first-value, API ergonomics, documentation quality",
                    weight: 1.2,
                },
                KpiTemplate {
                    id: "platform_reliability",
                    label: "Platform Reliability",
                    description: "Uptime, latency SLOs, and graceful degradation",
                    weight: 1.1,
                },
                KpiTemplate {
                    id: "ecosystem_growth",
                    label: "Ecosystem Growth",
                    description: "Community adoption, plugin ecosystem, integrations",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "security_trust",
                    label: "Security & Trust",
                    description: "Security posture, supply chain integrity, compliance",
                    weight: 1.0,
                },
            ],
        },
        DomainProfile {
            id: "generic",
            label: "Generic / Mixed",
            description: "Cross-functional or general-purpose team",
            kpis: vec![
                KpiTemplate {
                    id: "output_quality",
                    label: "Output Quality",
                    description: "Deliver high-quality, well-tested, maintainable work",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "delivery_speed",
                    label: "Delivery Speed",
                    description: "Complete work on time and with predictable velocity",
                    weight: 1.0,
                },
                KpiTemplate {
                    id: "strategic_alignment",
                    label: "Strategic Alignment",
                    description: "Work directly contributes to top business objectives",
                    weight: 1.0,
                },
            ],
        },
    ]
}

pub fn run(args: SetupArgs) -> Result<()> {
    let dir = &args.dir;
    let config_path = dir.join("meridian.toml");

    println!("Meridian Setup Wizard");
    println!("----------------------");
    if config_path.exists() {
        println!(
            "Warning: {} already exists and will be overwritten.",
            config_path.display()
        );
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Continue?")
            .default(false)
            .interact()?;
        if !proceed {
            println!("Aborted.");
            return Ok(());
        }
    } else {
        println!("Creating: {}", config_path.display());
    }
    println!();

    // Step 1: detect and choose source
    let source = pick_source(dir)?;

    // Step 2: choose domain profile
    let profiles = domain_profiles();
    let profile_labels: Vec<String> = profiles
        .iter()
        .map(|p| format!("{} — {}", p.label, p.description))
        .collect();

    let domain_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your domain profile")
        .items(&profile_labels)
        .default(0)
        .interact()?;
    let profile = &profiles[domain_idx];
    println!("  Using profile: {}", profile.label);
    println!();

    // Step 3: pick KPIs from the profile
    let kpi_labels: Vec<String> = profile
        .kpis
        .iter()
        .map(|k| format!("{} — {}", k.label, k.description))
        .collect();
    let all_idxs: Vec<bool> = vec![true; kpi_labels.len()];
    let selected_idxs = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select KPIs to track (space to toggle, enter to confirm)")
        .items(&kpi_labels)
        .defaults(&all_idxs)
        .interact()?;

    if selected_idxs.is_empty() {
        anyhow::bail!("At least one KPI is required. Re-run `meridian setup` to configure.");
    }

    let selected_kpis: Vec<&KpiTemplate> =
        selected_idxs.iter().map(|&i| &profile.kpis[i]).collect();

    println!("  Selected {} KPI(s).", selected_kpis.len());
    println!();

    // Step 4: write meridian.toml
    let toml = build_toml(&source, profile, &selected_kpis);
    std::fs::write(&config_path, &toml)?;
    println!("Written: {}", config_path.display());
    println!();

    // Step 5: optionally run analysis now
    let run_now = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Run `meridian analyze` now to see your first report?")
        .default(true)
        .interact()?;

    if run_now {
        println!();
        let analyze_args = super::analyze::AnalyzeArgs {
            source: None,
            path: None,
            format: None,
        };
        super::analyze::run(analyze_args, &config_path)?;
    } else {
        println!(
            "\nAll set! Run `meridian analyze` from {} to see your report.",
            dir.display()
        );
        println!("Run `meridian suggest` to get AI recommendations for low-KPI areas.");
    }

    Ok(())
}

enum SourceChoice {
    Ta(PathBuf),
    ClaudeCode(PathBuf),
    Jsonl,
}

fn pick_source(dir: &Path) -> Result<SourceChoice> {
    let ta_vel = TaSource::discover(dir);
    let cc_dir = claude_code::default_projects_dir();
    let has_cc = cc_dir.exists();

    let mut choices: Vec<&str> = Vec::new();
    if ta_vel.is_some() {
        choices.push("Trusted Autonomy (.ta/velocity-history.jsonl)");
    }
    if has_cc {
        choices.push("Claude Code (~/.claude/projects/)");
    }
    choices.push("Generic JSONL file");

    let idx = if choices.len() == 1 {
        println!("Source: {}", choices[0]);
        0
    } else {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select your effort data source")
            .items(&choices)
            .default(0)
            .interact()?
    };

    // Map index back to the right choice accounting for which options were present
    let mut cursor = 0usize;
    if let Some(vel) = ta_vel {
        if idx == cursor {
            println!(
                "  Using TA velocity: {}",
                vel.parent().unwrap_or(vel.as_path()).display()
            );
            return Ok(SourceChoice::Ta(
                vel.parent()
                    .and_then(|p| p.parent())
                    .unwrap_or(dir)
                    .to_path_buf(),
            ));
        }
        cursor += 1;
    }
    if has_cc {
        if idx == cursor {
            println!("  Using Claude Code: {}", cc_dir.display());
            return Ok(SourceChoice::ClaudeCode(cc_dir));
        }
        cursor += 1;
    }
    let _ = cursor;
    Ok(SourceChoice::Jsonl)
}

fn build_toml(source: &SourceChoice, profile: &DomainProfile, kpis: &[&KpiTemplate]) -> String {
    let mut out = String::new();

    writeln!(out, "# meridian.toml — generated by `meridian setup`").unwrap();
    writeln!(
        out,
        "# Edit to add custom categories, adjust KPI weights, or change source."
    )
    .unwrap();
    writeln!(out).unwrap();

    match source {
        SourceChoice::Ta(root) => {
            writeln!(out, "[source]").unwrap();
            writeln!(out, "ta_project_root = {:?}", root.to_string_lossy()).unwrap();
        }
        SourceChoice::ClaudeCode(dir) => {
            writeln!(out, "[source]").unwrap();
            writeln!(out, "claude_code_dir = {:?}", dir.to_string_lossy()).unwrap();
        }
        SourceChoice::Jsonl => {
            writeln!(out, "[source]").unwrap();
            writeln!(out, "# jsonl = \"records.jsonl\"").unwrap();
        }
    }
    writeln!(out).unwrap();

    writeln!(out, "[report]").unwrap();
    writeln!(out, "format = \"table\"").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "# KPIs for domain: {} ({})", profile.label, profile.id).unwrap();
    for kpi in kpis {
        writeln!(out, "[[kpis]]").unwrap();
        writeln!(out, "id = {:?}", kpi.id).unwrap();
        writeln!(out, "label = {:?}", kpi.label).unwrap();
        writeln!(out, "description = {:?}", kpi.description).unwrap();
        writeln!(out, "weight = {}", kpi.weight).unwrap();
        writeln!(out).unwrap();
    }

    writeln!(out, "[suggest]").unwrap();
    writeln!(out, "threshold = 0.25").unwrap();
    writeln!(out, "sample_size = 5").unwrap();
    writeln!(out, "model = \"claude-haiku-4-5-20251001\"").unwrap();
    writeln!(
        out,
        "# api_key = \"...\"  # or set ANTHROPIC_API_KEY env var"
    )
    .unwrap();

    out
}
