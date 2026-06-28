use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use console::style;
use dialoguer::{Confirm, Input, MultiSelect};
use meridian_config::{KpiConfig, MeridianConfig};
use meridian_core::metric::Metric;
use meridian_report::auth::AuthMethod;
use meridian_report::pm::{
    self, ConcernSeverity, DraftKpi, PanelistVerdict, PmInterview, PANELISTS,
};
use std::path::Path;

// ── CLI types ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct PmArgs {
    #[command(subcommand)]
    pub command: PmCmd,
}

#[derive(Subcommand)]
pub enum PmCmd {
    /// Interview mode: collect priorities, generate KPIs, run red-team review, write meridian.toml
    Init(PmInitArgs),
    /// Re-run red-team review on existing KPIs from meridian.toml and apply resolutions
    Refine(PmRefineArgs),
}

#[derive(Args)]
pub struct PmInitArgs {
    /// Claude model to use (default: claude-haiku-4-5-20251001)
    #[arg(long, default_value = "claude-haiku-4-5-20251001")]
    pub model: String,

    /// Skip red-team review and write KPIs directly
    #[arg(long)]
    pub yes: bool,

    /// Write output to this path instead of meridian.toml
    #[arg(long)]
    pub output: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct PmRefineArgs {
    /// Claude model to use (default: claude-haiku-4-5-20251001)
    #[arg(long, default_value = "claude-haiku-4-5-20251001")]
    pub model: String,

    /// Accept all high-severity suggestions automatically without prompting
    #[arg(long)]
    pub yes: bool,
}

// ── Entry point ────────────────────────────────────────────────────────────────

pub fn run(args: PmArgs, config_path: &Path) -> Result<()> {
    match args.command {
        PmCmd::Init(a) => run_init(a, config_path),
        PmCmd::Refine(a) => run_refine(a, config_path),
    }
}

// ── Init flow ──────────────────────────────────────────────────────────────────

fn run_init(args: PmInitArgs, config_path: &Path) -> Result<()> {
    let auth = AuthMethod::resolve(None).ok_or_else(|| {
        anyhow::anyhow!(
            "Set ANTHROPIC_API_KEY or install the claude CLI (https://claude.ai/code) to use `meridian pm init`."
        )
    })?;

    println!(
        "\n{}",
        style("Meridian Virtual PM — KPI Interview").bold().cyan()
    );
    println!("Answer a few questions and we'll generate KPIs with metrics for your team.\n");

    let interview = collect_interview()?;

    println!("\n{} Generating KPIs...", style("...").dim());
    let draft_kpis = pm::generate_draft_kpis(&interview, &auth, &args.model)?;

    println!("\n{}\n", style("Draft KPIs").bold().underlined());
    print_draft_kpis(&draft_kpis);

    let final_kpis = if args.yes {
        draft_kpis
    } else {
        let verdicts = run_red_team_interactive(&draft_kpis, &interview, &auth, &args.model)?;
        let consensus = pm::consensus(&verdicts);
        print_consensus_summary(&consensus);

        if consensus.high_severity.is_empty() && consensus.rejected == 0 {
            println!(
                "\n{} No blocking concerns. Finalizing KPIs.",
                style("OK").green()
            );
            draft_kpis
        } else {
            let resolutions = collect_resolutions(&verdicts)?;
            if resolutions.is_empty() {
                draft_kpis
            } else {
                println!(
                    "\n{} Regenerating KPIs with resolutions...",
                    style("...").dim()
                );
                pm::finalize_with_resolutions(&draft_kpis, &resolutions, &auth, &args.model)?
            }
        }
    };

    let output_path = args.output.as_deref().unwrap_or(config_path);

    write_kpis_to_config(output_path, &final_kpis)?;
    println!(
        "\n{} KPIs written to {}",
        style("Done").green().bold(),
        output_path.display()
    );
    println!(
        "Run {} to see alignment scores.",
        style("meridian report").bold()
    );

    Ok(())
}

// ── Refine flow ────────────────────────────────────────────────────────────────

fn run_refine(args: PmRefineArgs, config_path: &Path) -> Result<()> {
    let auth = AuthMethod::resolve(None).ok_or_else(|| {
        anyhow::anyhow!(
            "Set ANTHROPIC_API_KEY or install the claude CLI (https://claude.ai/code) to use `meridian pm refine`."
        )
    })?;

    let config = MeridianConfig::load(config_path)
        .map_err(|e| anyhow::anyhow!("{e}\nRun `meridian pm init` first to create a config."))?;

    if config.kpis.is_empty() {
        bail!(
            "No KPIs defined in {}. Run `meridian pm init` first.",
            config_path.display()
        );
    }

    let taxonomy = config.taxonomy();
    let draft_kpis: Vec<DraftKpi> = taxonomy
        .kpis
        .iter()
        .map(|k| DraftKpi {
            id: k.id.clone(),
            label: k.label.clone(),
            description: k.description.clone(),
            weight: k.weight,
            metrics: k.metrics.clone(),
            rationale: String::new(),
        })
        .collect();

    let interview = PmInterview::default();

    println!(
        "\n{} Running red-team review on {} KPIs...",
        style("...").dim(),
        draft_kpis.len()
    );

    let verdicts = run_red_team_interactive(&draft_kpis, &interview, &auth, &args.model)?;
    let consensus = pm::consensus(&verdicts);
    print_consensus_summary(&consensus);

    if consensus.high_severity.is_empty() && consensus.rejected == 0 {
        println!("\n{} No blocking concerns found.", style("OK").green());
        return Ok(());
    }

    let resolutions = if args.yes {
        consensus
            .high_severity
            .iter()
            .map(|(role, c)| {
                (
                    format!("[{}] {}", role, c.concern),
                    c.suggested_fix
                        .clone()
                        .unwrap_or_else(|| "Accept suggestion".to_string()),
                )
            })
            .collect()
    } else {
        collect_resolutions(&verdicts)?
    };

    if resolutions.is_empty() {
        println!("No changes applied.");
        return Ok(());
    }

    println!("\n{} Regenerating KPIs...", style("...").dim());
    let final_kpis = pm::finalize_with_resolutions(&draft_kpis, &resolutions, &auth, &args.model)?;

    write_kpis_to_config(config_path, &final_kpis)?;
    println!(
        "\n{} KPIs updated in {}",
        style("Done").green().bold(),
        config_path.display()
    );

    Ok(())
}

// ── Interview helpers ──────────────────────────────────────────────────────────

fn collect_interview() -> Result<PmInterview> {
    let industry: String = Input::new()
        .with_prompt("Industry / vertical (e.g. SaaS, fintech, game studio)")
        .allow_empty(true)
        .interact_text()?;

    let team_size_str: String = Input::new()
        .with_prompt("Approximate team size (press Enter to skip)")
        .allow_empty(true)
        .interact_text()?;

    let team_size: Option<u32> = team_size_str.trim().parse().ok();

    println!("\nEnter your top strategic priorities (one per line, empty line to finish):");
    let priorities = collect_lines("Priority")?;

    println!("\nDescribe your goals in your own words (one per line, empty line to finish):");
    let goals = collect_lines("Goal")?;

    println!("\nAny constraints to respect? e.g. budget limits, team capacity (one per line, empty to skip):");
    let constraints = collect_lines("Constraint")?;

    Ok(PmInterview {
        industry: if industry.is_empty() {
            None
        } else {
            Some(industry)
        },
        team_size,
        priorities,
        goals,
        constraints,
    })
}

fn collect_lines(label: &str) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let mut index = 1;
    loop {
        let line: String = Input::new()
            .with_prompt(format!("{label} {index} (Enter to finish)"))
            .allow_empty(true)
            .interact_text()?;
        if line.is_empty() {
            break;
        }
        lines.push(line);
        index += 1;
    }
    Ok(lines)
}

// ── Red team display ───────────────────────────────────────────────────────────

fn run_red_team_interactive(
    kpis: &[DraftKpi],
    interview: &PmInterview,
    auth: &AuthMethod,
    model: &str,
) -> Result<Vec<PanelistVerdict>> {
    println!(
        "\n{} Running {}-panelist red-team review...",
        style("Red team").yellow().bold(),
        PANELISTS.len()
    );
    for p in PANELISTS {
        print!("  {}", p.role);
    }
    println!();

    let verdicts = pm::run_red_team(kpis, interview, auth, model)?;

    for v in &verdicts {
        println!(
            "  {} — {}",
            style(&v.panelist_role).bold(),
            match v.decision {
                pm::PanelistDecision::Approve => style("APPROVE").green().to_string(),
                pm::PanelistDecision::Challenge => style("CHALLENGE").yellow().to_string(),
                pm::PanelistDecision::Reject => style("REJECT").red().to_string(),
            }
        );
    }

    Ok(verdicts)
}

fn print_consensus_summary(c: &pm::ConsensusVerdict) {
    println!("\n{}", style("Red Team Summary").bold().underlined());
    println!(
        "  Approve: {}  Challenge: {}  Reject: {}",
        style(c.approved).green(),
        style(c.challenged).yellow(),
        style(c.rejected).red()
    );

    if !c.high_severity.is_empty() {
        println!("\n{}", style("High-Severity Concerns:").red().bold());
        for (role, concern) in &c.high_severity {
            println!("  [{}] {}", style(role).bold(), concern.concern);
            if let Some(fix) = &concern.suggested_fix {
                println!("       Suggestion: {}", style(fix).dim());
            }
        }
    }
}

// ── Resolution helpers ─────────────────────────────────────────────────────────

fn collect_resolutions(verdicts: &[PanelistVerdict]) -> Result<Vec<(String, String)>> {
    let all_concerns: Vec<(String, String, ConcernSeverity, Option<String>)> = verdicts
        .iter()
        .flat_map(|v| {
            v.concerns.iter().map(|c| {
                (
                    v.panelist_role.clone(),
                    c.concern.clone(),
                    c.severity,
                    c.suggested_fix.clone(),
                )
            })
        })
        .collect();

    if all_concerns.is_empty() {
        return Ok(vec![]);
    }

    println!("\n{}", style("Review Concerns").bold().underlined());
    println!("Select the concerns you want to address (space to select, enter to confirm):\n");

    let items: Vec<String> = all_concerns
        .iter()
        .map(|(role, concern, severity, _)| {
            format!("[{}] [{:?}] {}: {}", role, severity, role, concern)
        })
        .collect();

    let defaults: Vec<bool> = all_concerns
        .iter()
        .map(|(_, _, sev, _)| *sev == ConcernSeverity::High)
        .collect();

    let selected = MultiSelect::new()
        .with_prompt("Use space to toggle, enter to confirm")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    if selected.is_empty() {
        println!("No concerns selected — KPIs unchanged.");
        return Ok(vec![]);
    }

    let mut resolutions = Vec::new();
    for idx in selected {
        let (role, concern, _, suggested) = &all_concerns[idx];
        println!("\n{}: {}", style(role).bold(), concern);
        if let Some(fix) = suggested {
            println!("  Suggested fix: {}", style(fix).dim());
        }

        let default_response = suggested
            .clone()
            .unwrap_or_else(|| "Accept and address".to_string());
        let response: String = Input::new()
            .with_prompt("Your response / how to address this")
            .default(default_response)
            .interact_text()?;

        resolutions.push((format!("[{}] {}", role, concern), response));
    }

    Ok(resolutions)
}

// ── Display helpers ────────────────────────────────────────────────────────────

fn print_draft_kpis(kpis: &[DraftKpi]) {
    for kpi in kpis {
        println!("  {} — {}", style(&kpi.label).bold(), kpi.description);
        println!("    Rationale: {}", style(&kpi.rationale).dim());
        if !kpi.metrics.is_empty() {
            println!("    Metrics:");
            for m in &kpi.metrics {
                println!(
                    "      - {} ({} {}, target: {}, source: {})",
                    m.label, m.unit, m.frequency, m.target, m.source
                );
            }
        }
        println!();
    }
}

// ── Config writer ──────────────────────────────────────────────────────────────

fn write_kpis_to_config(path: &Path, kpis: &[DraftKpi]) -> Result<()> {
    let mut config = if path.exists() {
        MeridianConfig::load(path).unwrap_or_default()
    } else {
        MeridianConfig::default()
    };

    let confirm = if config.kpis.is_empty() {
        true
    } else {
        Confirm::new()
            .with_prompt(format!(
                "{} already has {} KPI(s). Replace them?",
                path.display(),
                config.kpis.len()
            ))
            .default(true)
            .interact()?
    };

    if !confirm {
        println!("KPIs not written.");
        return Ok(());
    }

    config.kpis = kpis
        .iter()
        .map(|k| KpiConfig {
            id: k.id.clone(),
            label: k.label.clone(),
            description: k.description.clone(),
            weight: k.weight,
            metrics: k
                .metrics
                .iter()
                .map(|m| Metric {
                    id: m.id.clone(),
                    label: m.label.clone(),
                    unit: m.unit.clone(),
                    target: m.target.clone(),
                    source: m.source.clone(),
                    frequency: m.frequency.clone(),
                })
                .collect(),
        })
        .collect();

    let toml_str = toml::to_string_pretty(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize meridian.toml: {e}"))?;

    std::fs::write(path, toml_str)
        .map_err(|e| anyhow::anyhow!("Failed to write {}: {e}", path.display()))?;

    Ok(())
}
