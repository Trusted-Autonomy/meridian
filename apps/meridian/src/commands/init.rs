use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct InitArgs {
    /// Directory to create meridian.toml in
    #[arg(default_value = ".")]
    pub dir: std::path::PathBuf,
}

const EXAMPLE_CONFIG: &str = r#"# meridian.toml
# Run `meridian analyze` to analyze effort records.

# ─── Data sources ─────────────────────────────────────────────────────────────
# Auto-detected when run from a TA project directory.
# Uncomment to set explicitly:

[source]
# ta_project_root = "/path/to/your/TrustedAutonomy"
# jsonl = "records.jsonl"   # standalone: {id, title, tokens_input?, tokens_output?, seconds?}

# ─── Output ───────────────────────────────────────────────────────────────────
[report]
format = "table"   # table | json | csv

# ─── KPI Definitions ──────────────────────────────────────────────────────────
# Meridian scores each activity against each KPI using keyword cosine similarity.
# No API calls needed — pure local computation.

[[kpis]]
id = "engineering_velocity"
label = "Engineering Velocity"
description = "Activities that improve team ability to ship quality software faster — code, tests, CI/CD, refactoring"
weight = 1.0

[[kpis]]
id = "revenue_growth"
label = "Revenue Growth"
description = "Activities that directly drive or support revenue — sales, marketing, customer acquisition, product features"
weight = 1.0

[[kpis]]
id = "risk_reduction"
label = "Risk Reduction"
description = "Activities that reduce security, compliance, operational, or business risk — audits, hardening, documentation"
weight = 0.8

[[kpis]]
id = "customer_satisfaction"
label = "Customer Satisfaction"
description = "Activities that improve user experience, onboarding, support, and product quality"
weight = 0.9

# ─── Custom Categories (optional) ─────────────────────────────────────────────
# Meridian ships 10 built-in categories. Override or extend here.

# [[categories]]
# id = "ai_infra"
# label = "AI Infrastructure"
# description = "Building and maintaining AI agent pipelines, LLM integrations, model serving"
# keywords = ["agent", "llm", "model", "prompt", "inference", "embedding"]
"#;

pub fn run(args: InitArgs) -> Result<()> {
    let path = args.dir.join("meridian.toml");
    if path.exists() {
        anyhow::bail!(
            "{} already exists — delete it first to reinitialize",
            path.display()
        );
    }
    std::fs::write(&path, EXAMPLE_CONFIG)?;
    println!("Created {}", path.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit meridian.toml — define your KPIs under [[kpis]]");
    println!("  2. meridian analyze                             # from a TA project dir");
    println!("  3. meridian analyze --source jsonl --path records.jsonl  # standalone");
    Ok(())
}
