use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(
    name = "meridian",
    version,
    about = "Token spend and effort analytics with KPI alignment"
)]
struct Cli {
    /// Path to meridian.toml config
    #[arg(long, global = true, default_value = "meridian.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Analyze effort records and print a categorized report
    Analyze(commands::analyze::AnalyzeArgs),
    /// Create a meridian.toml with example KPIs in the target directory
    Init(commands::init::InitArgs),
    /// Virtual PM mode: interview → generate KPIs → red-team review → write meridian.toml
    ///
    /// Subcommands: init (generate KPIs from interview), refine (re-run red team on existing KPIs)
    ///
    /// Studio integration note: `pm init --yes` can be run headlessly to generate a draft;
    /// the interactive review step is skipped and the full review can be presented in Studio UI.
    Pm(commands::pm::PmArgs),
    /// Expert panel evaluation report for a time window (default: last 7 days)
    Report(commands::report::ReportArgs),
    /// Interactive wizard: detect source, pick domain profile, write meridian.toml
    Setup(commands::setup::SetupArgs),
    /// Suggest how to better align low-scoring categories to KPIs
    Suggest(commands::suggest::SuggestArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Analyze(args) => commands::analyze::run(args, &cli.config),
        Cmd::Init(args) => commands::init::run(args),
        Cmd::Pm(args) => commands::pm::run(args, &cli.config),
        Cmd::Report(args) => commands::report::run(args, &cli.config),
        Cmd::Setup(args) => commands::setup::run(args),
        Cmd::Suggest(args) => commands::suggest::run(args, &cli.config),
    }
}
