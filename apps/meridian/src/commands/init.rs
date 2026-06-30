use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct InitArgs {
    /// Directory to create meridian.toml in
    #[arg(default_value = ".")]
    pub dir: std::path::PathBuf,

    /// Business type preset: software (default), saas, agency, game-studio, enterprise, startup
    #[arg(long, default_value = "software")]
    pub preset: String,
}

// ── Preset configs ────────────────────────────────────────────────────────────

const PRESET_SOFTWARE: &str = r#"# meridian.toml — software team preset
# Run `meridian analyze` to analyze effort records.

[source]
# ta_project_root = "/path/to/your/project"   # TA project (auto-detected from cwd)
# jsonl = "work.jsonl"                        # standalone append-log
# claude_code_dir = "~/.claude/projects"      # Claude Code session history

[report]
format = "table"   # table | json | csv

[[kpis]]
id = "engineering_velocity"
label = "Engineering Velocity"
description = "Activities that improve team ability to ship quality software faster — code, tests, CI/CD, refactoring, architecture"
weight = 1.0

[[kpis]]
id = "revenue_growth"
label = "Revenue Growth"
description = "Activities that directly drive or support revenue — sales, marketing, customer acquisition, product features users pay for"
weight = 1.0

[[kpis]]
id = "risk_reduction"
label = "Risk Reduction"
description = "Activities that reduce security, compliance, operational, or business risk — audits, hardening, documentation, reliability"
weight = 0.8

[[kpis]]
id = "customer_satisfaction"
label = "Customer Satisfaction"
description = "Activities that improve user experience, onboarding, support responsiveness, and product quality"
weight = 0.9

# Built-in categories cover software teams well — no [[categories]] override needed.
"#;

const PRESET_SAAS: &str = r#"# meridian.toml — SaaS product company preset
# Run `meridian analyze` to analyze effort records.

[source]
# ta_project_root = "/path/to/your/project"
# jsonl = "work.jsonl"
# claude_code_dir = "~/.claude/projects"

[report]
format = "table"

[[kpis]]
id = "product_velocity"
label = "Product Velocity"
description = "Shipping features, improvements, and fixes that customers experience — code, product decisions, design, releases"
weight = 1.0

[[kpis]]
id = "arr_growth"
label = "ARR Growth"
description = "Activities that expand annual recurring revenue — new features, sales, marketing, pricing, expansion"
weight = 1.0

[[kpis]]
id = "churn_reduction"
label = "Churn Reduction"
description = "Activities that improve retention — customer success, support, onboarding, reliability, product quality"
weight = 0.9

[[kpis]]
id = "technical_health"
label = "Technical Health"
description = "Reducing technical debt, improving observability, security hardening, infrastructure reliability, test coverage"
weight = 0.7
"#;

const PRESET_AGENCY: &str = r#"# meridian.toml — agency / service business preset
# Run `meridian analyze` to analyze effort records.

[source]
# jsonl = "work.jsonl"
# claude_code_dir = "~/.claude/projects"

[report]
format = "table"

[[kpis]]
id = "client_delivery"
label = "Client Delivery"
description = "Work that delivers value directly to clients — project execution, creative output, deliverable production, revisions"
weight = 1.0

[[kpis]]
id = "revenue_growth"
label = "Revenue Growth"
description = "Business development, proposals, pitches, new account acquisition, upsells, partnerships"
weight = 1.0

[[kpis]]
id = "creative_quality"
label = "Creative Quality"
description = "Work that raises the craft and quality of output — creative direction, design iteration, editorial review, research"
weight = 0.9

[[kpis]]
id = "operational_efficiency"
label = "Operational Efficiency"
description = "Process improvement, tooling, templates, team coordination, reducing friction in delivery"
weight = 0.7

[[categories]]
id = "client_work"
label = "Client Work"
description = "Executing billable client projects — design, copy, development, strategy, deliverables"
keywords = ["client", "deliverable", "brief", "revision", "project", "scope", "account"]

[[categories]]
id = "creative"
label = "Creative & Production"
description = "Creative development, ideation, design, writing, editing, visual production"
keywords = ["design", "copy", "write", "edit", "visual", "creative", "content", "produce"]

[[categories]]
id = "biz_dev"
label = "Business Development"
description = "Proposals, pitches, new business, partnerships, networking, contract negotiation"
keywords = ["proposal", "pitch", "prospect", "contract", "partnership", "rfp"]

[[categories]]
id = "project_mgmt"
label = "Project Management"
description = "Planning, coordination, timelines, resourcing, status, client communication"
keywords = ["plan", "timeline", "resource", "status", "coordination", "kickoff"]

[[categories]]
id = "ops"
label = "Operations"
description = "Internal tooling, process improvement, finance, HR, admin, reporting"
keywords = ["process", "template", "tool", "admin", "finance", "report", "internal"]
"#;

const PRESET_GAME_STUDIO: &str = r#"# meridian.toml — game studio preset
# Run `meridian analyze` to analyze effort records.

[source]
# ta_project_root = "/path/to/your/project"
# jsonl = "work.jsonl"

[report]
format = "table"

[[kpis]]
id = "player_experience"
label = "Player Experience"
description = "Work that directly improves how players feel and engage — gameplay, UX, balance, performance, feel"
weight = 1.0

[[kpis]]
id = "feature_completion"
label = "Feature Completion"
description = "Shipping game systems, content, and features on schedule — implementation, art, audio, testing"
weight = 1.0

[[kpis]]
id = "technical_performance"
label = "Technical Performance"
description = "Frame rate, load times, memory, stability, platform compliance, crash reduction"
weight = 0.9

[[kpis]]
id = "revenue"
label = "Revenue & Growth"
description = "Monetization, user acquisition, store presence, live ops, community, marketing"
weight = 0.8

[[categories]]
id = "gameplay"
label = "Gameplay & Mechanics"
description = "Game mechanics, player systems, AI behavior, combat, progression, balance"
keywords = ["gameplay", "mechanic", "system", "player", "balance", "combat", "progression", "ai"]

[[categories]]
id = "art"
label = "Art & Animation"
description = "Character art, environment, animation, rigging, VFX, materials, textures, UI art"
keywords = ["art", "animation", "rig", "texture", "material", "vfx", "environment", "character"]

[[categories]]
id = "audio"
label = "Audio"
description = "Sound effects, music, voice acting, audio integration, mixing, audio systems"
keywords = ["audio", "sound", "music", "voice", "sfx", "mix", "wwise", "fmod"]

[[categories]]
id = "engine"
label = "Engine & Tools"
description = "Engine work, editor tooling, build pipeline, rendering, optimization, platform"
keywords = ["engine", "renderer", "shader", "pipeline", "editor", "tool", "build", "platform"]

[[categories]]
id = "production"
label = "Production & QA"
description = "Milestones, scheduling, QA, bug tracking, certification, submission, release"
keywords = ["milestone", "qa", "bug", "certification", "release", "schedule", "submission"]

[[categories]]
id = "live_ops"
label = "Live Ops & Marketing"
description = "Live events, updates, community, store, monetization, user acquisition, marketing"
keywords = ["live", "event", "community", "store", "monetization", "patch", "update", "dlc"]
"#;

const PRESET_ENTERPRISE: &str = r#"# meridian.toml — enterprise team preset
# Run `meridian analyze` to analyze effort records.

[source]
# ta_project_root = "/path/to/your/project"
# jsonl = "work.jsonl"

[report]
format = "table"

[[kpis]]
id = "compliance_adherence"
label = "Compliance & Governance"
description = "Activities that maintain or improve regulatory compliance, audit readiness, policy enforcement, data governance"
weight = 1.0

[[kpis]]
id = "operational_efficiency"
label = "Operational Efficiency"
description = "Process automation, tooling improvements, reducing manual toil, cost reduction, workflow optimization"
weight = 1.0

[[kpis]]
id = "risk_reduction"
label = "Risk Reduction"
description = "Security hardening, vulnerability remediation, disaster recovery, access control, incident response"
weight = 1.0

[[kpis]]
id = "stakeholder_value"
label = "Stakeholder Value"
description = "Work visible to leadership or customers — reporting, demos, strategic initiatives, executive communication"
weight = 0.8

# Built-in categories are suitable for enterprise engineering teams.
"#;

const PRESET_STARTUP: &str = r#"# meridian.toml — early-stage startup preset
# Run `meridian analyze` to analyze effort records.

[source]
# ta_project_root = "/path/to/your/project"
# jsonl = "work.jsonl"
# claude_code_dir = "~/.claude/projects"

[report]
format = "table"

[[kpis]]
id = "revenue_growth"
label = "Revenue Growth"
description = "Directly drives ARR, closes deals, acquires users — sales, marketing, billing features, pricing, expansion"
weight = 1.0

[[kpis]]
id = "product_market_fit"
label = "Product-Market Fit"
description = "Learning what customers need and building it — user research, discovery, core feature development, rapid iteration"
weight = 1.0

[[kpis]]
id = "engineering_velocity"
label = "Engineering Velocity"
description = "Shipping fast — features, fixes, CI/CD, developer experience, reducing friction in the build-measure-learn cycle"
weight = 0.9

[[kpis]]
id = "runway_extension"
label = "Runway Extension"
description = "Work that stretches runway — cost reduction, automation, efficiency, avoiding unnecessary infrastructure spend"
weight = 0.7
"#;

// ── Preset dispatch ───────────────────────────────────────────────────────────

fn config_for_preset(preset: &str) -> Result<&'static str> {
    match preset.to_lowercase().replace('_', "-").as_str() {
        "software" => Ok(PRESET_SOFTWARE),
        "saas" => Ok(PRESET_SAAS),
        "agency" => Ok(PRESET_AGENCY),
        "game-studio" | "game" => Ok(PRESET_GAME_STUDIO),
        "enterprise" => Ok(PRESET_ENTERPRISE),
        "startup" => Ok(PRESET_STARTUP),
        other => anyhow::bail!(
            "Unknown preset '{}'. Available: software (default), saas, agency, game-studio, enterprise, startup",
            other
        ),
    }
}

pub fn run(args: InitArgs) -> Result<()> {
    let path = args.dir.join("meridian.toml");
    if path.exists() {
        anyhow::bail!(
            "{} already exists — delete it first to reinitialize",
            path.display()
        );
    }
    let config = config_for_preset(&args.preset)?;
    std::fs::write(&path, config)?;
    println!("Created {} (preset: {})", path.display(), args.preset);
    println!();
    println!("Next steps:");
    println!("  1. Edit meridian.toml — tune KPI descriptions and weights for your team");
    println!("  2. meridian analyze                                    # TA project (auto-detected)");
    println!("  3. meridian analyze --source jsonl --path work.jsonl  # standalone log");
    println!("  4. meridian analyze --source claude_code              # Claude Code sessions");
    Ok(())
}
