# Meridian Quickstart

Track where your AI effort actually goes — categorize sessions by work type, score them against your team's KPIs, and get AI-powered suggestions for better alignment. Works standalone with Claude Code, Cursor, or any AI tool that produces logs.

---

## 5-Minute Setup

### 1. Install

```bash
# From source (requires Rust)
git clone https://github.com/Trusted-Autonomy/meridian
cd meridian
cargo install --path apps/meridian

# Or build locally
cargo build --release -p meridian
# Binary: target/release/meridian
```

### 2. Run the setup wizard

```bash
meridian setup
```

The wizard:
1. Detects your data source (Claude Code, Trusted Autonomy, or custom JSONL)
2. Asks you to pick a domain profile (SaaS, Gamedev, Fintech, Devtools, etc.)
3. Lets you select which KPIs matter for your team
4. Writes `meridian.toml` to the current directory
5. Optionally runs your first analysis immediately

### 3. Analyze

```bash
meridian analyze
```

Prints a table showing how your effort is distributed across categories (Code, Ops, Docs, Security, etc.) and how each category scores against your KPIs.

### 4. Get improvement suggestions

```bash
meridian suggest
```

Identifies category × KPI pairs with low alignment and uses Claude to suggest concrete ways to reframe or redirect work.

Requires `ANTHROPIC_API_KEY` (or set `api_key` in `meridian.toml`).

---

## Common Workflows

### Analyze Claude Code sessions

```bash
# Auto-detects ~/.claude/projects/
meridian analyze --source claude-code

# Or point at a specific projects directory
meridian analyze --source claude-code --path /path/to/.claude/projects
```

### Analyze a Trusted Autonomy project

```bash
cd /path/to/your/ta-project
meridian analyze --source ta
```

### Analyze any tool via generic JSONL

```bash
meridian analyze --source jsonl --path my-sessions.jsonl
```

See [sources/standalone.md](sources/standalone.md) for the JSONL format.

### Export to JSON or CSV

```bash
meridian analyze --format json > report.json
meridian analyze --format csv > report.csv
```

---

## Configuration (meridian.toml)

`meridian setup` writes this for you, but you can edit it manually:

```toml
[source]
# Pick one:
claude_code_dir = "~/.claude/projects"   # Claude Code
# ta_project_root = "/path/to/project"   # Trusted Autonomy
# jsonl = "sessions.jsonl"               # Custom

[report]
format = "table"   # table | json | csv

[[kpis]]
id = "eng_velocity"
label = "Engineering Velocity"
description = "Ship quality features faster"
weight = 1.0

[[kpis]]
id = "product_quality"
label = "Product Quality"
description = "Reduce defects and improve stability"
weight = 1.0

[suggest]
threshold = 0.25       # KPI score below this triggers a suggestion
sample_size = 5        # Number of example sessions to include as context
model = "claude-haiku-4-5-20251001"
# api_key = "..."      # or ANTHROPIC_API_KEY env var
```

---

## Understanding the Report

```
Category                 Sessions   Effort (pts)   KPI Scores
-----------------------------------------------------------------
Code Implementation         42          1850        EV: 72%  PQ: 68%
Ops & Infrastructure        18           820        EV: 45%  PQ: 55%
Documentation                8           210        EV: 30%  PQ: 40%
```

- **Effort (pts)**: normalized effort points. For tokens: `(input/1000) + (output/1000 * 3)`. For TA velocity: raw seconds.
- **KPI Scores**: how well the work in that category aligns with each KPI based on keyword matching.
- **Low scores**: run `meridian suggest` to get actionable recommendations.

---

## Next Steps

- [Claude Code integration](sources/claude-code.md)
- [Cursor integration](sources/cursor.md)
- [Generic JSONL format](sources/standalone.md)
