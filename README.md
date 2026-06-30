# Meridian

Token spend and KPI alignment analytics for AI agent workflows.

Meridian answers: **Where is your team's AI effort going, and does it align with what matters?**

It classifies agent work by business category (code, PM, docs, sales, etc.), scores each category against your company KPIs, and — optionally — generates concrete suggestions for improving alignment.

**No external services required.** The default keyword scorer runs offline with zero API calls. TA is optional.

---

**Available as:**
- **MCP tool** — add to any Claude session with one command; your agent can query spend, analyze alignment, and get suggestions without leaving the conversation
- **Standalone CLI** — works with any JSONL work log, Claude Code session history, or any LLM usage export
- **Trusted Autonomy integration** — built-in via `ta meridian`; every agent goal run is automatically tracked and available for KPI analysis

---

## Quick start (MCP tool)

The fastest way to get Meridian working — add it to any Claude Code or Claude Desktop session.

```bash
# Install
cargo install --git https://github.com/Trusted-Autonomy/meridian

# Add to Claude Code (runs meridian serve as a local MCP server)
claude mcp add meridian -- meridian serve
```

Your agent can now call these tools directly in any session:

| Tool | What it does |
|------|-------------|
| `meridian_report` | Spend summary by category for a time window |
| `meridian_analyze` | Full classification breakdown with KPI alignment scores |
| `meridian_kpis` | Your configured KPIs with current alignment |
| `meridian_suggest` | Low-scoring category×KPI pairs flagged for attention |
| `meridian_summarize_title` | Extract a clean work title from raw prompt text |

## Quick start (standalone CLI)

No MCP, no TA required. Point Meridian at any JSONL work log or Claude Code session history.

```bash
# Create a config for your business type, then analyze
meridian init --preset software    # or: saas, agency, game-studio, enterprise, startup
meridian analyze --source claude_code                  # Claude Code sessions (~/.claude/projects)
meridian analyze --source jsonl --path work.jsonl      # your own work log
```

## Quick start (with Trusted Autonomy)

If you use TA, Meridian reads `.ta/velocity-history.jsonl` directly — every completed goal run is a record automatically.

```bash
# From any TA project root (auto-detected):
meridian analyze

# Or with an explicit path:
meridian analyze --source ta --path ~/development/MyProject

# Via the ta CLI (v0.17.0.12+):
ta meridian analyze
ta meridian suggest
```

## How it works: bi-prism analysis

Meridian runs two independent scoring passes — the **bi-prism** — giving two orthogonal views of every record:

What type of work is this?
```
                    +---------------------+
  work title -----> | Pass 1: Category    | --> "Code Implementation" (87% confidence)
                    | (what type?)        |
                    +---------------------+
```

Which KPIs does this work match with most closely?
```
                    +---------------------+
  work title -----> | Pass 2: KPI Align   | --> engineering_velocity: 0.82
                    | (how aligned?)      |     revenue_growth: 0.11  <- suggest!
                    +---------------------+
```

The two passes are independent — category classification doesn't influence KPI alignment scores. This keeps the model honest and makes per-category KPI breakdowns meaningful.

**Default scorer: keyword TF-IDF cosine** — zero API calls, works offline, no setup required. Upgrade to semantic embeddings (Voyage AI) for better accuracy on short or ambiguous titles.

### What the output looks like

```
Meridian - Effort Analysis
------------------------------------------------------------------------
Category                 Records       Effort   % Total
------------------------------------------------------------------------
Code Implementation           42      145320     48.1%
  KPI: engineering_velocity:82%  revenue_growth:14%
Operations & Infrastructure   18       62400     20.7%
  KPI: engineering_velocity:71%  revenue_growth:9%
Documentation                 12       41200     13.6%
  KPI: engineering_velocity:55%  revenue_growth:7%
Project Management             8       28800      9.5%
  KPI: engineering_velocity:43%  revenue_growth:31%
Research & Exploration         5       24000      7.9%
  KPI: engineering_velocity:38%  revenue_growth:22%
------------------------------------------------------------------------
Total: 85 records, 301720 effort points
```

```
$ meridian suggest

Low KPI Alignment (3 pairs below 25% threshold):
  Code Implementation x Revenue Growth - 14%
  Operations & Infrastructure x Revenue Growth - 9%
  Documentation x Revenue Growth - 7%

-- Code Implementation x Revenue Growth (14% alignment) --
  1. Prioritize features that directly appear in customer-facing changelogs -- frame
     implementation work around user value delivered, not internal refactoring.
  2. When fixing bugs, link each fix to a customer-reported issue or NPS signal --
     this surfaces the revenue-impact of reliability work.
  3. Allocate ~20% of implementation cycles to API/integration surface improvements
     that unblock sales engineering and partner integrations.
```

---

## Defining your taxonomy

Meridian's classification is driven by two things you control: **categories** (what type of work) and **KPIs** (what your business cares about).

### KPIs

KPIs are the business outcomes you want to track. Define them in `meridian.toml`. The `description` is the most important field — it's what the scorer matches work titles against.

```toml
[[kpis]]
id = "engineering_velocity"
label = "Engineering Velocity"
description = "Activities that improve team ability to ship quality software faster — code, tests, CI/CD, refactoring, architecture"
weight = 1.0

[[kpis]]
id = "revenue_growth"
label = "Revenue Growth"
description = "Activities that directly drive or support revenue — sales, marketing, customer acquisition, product features that users pay for"
weight = 1.0

[[kpis]]
id = "risk_reduction"
label = "Risk Reduction"
description = "Activities that reduce security, compliance, operational, or business risk — audits, hardening, documentation, backup systems"
weight = 0.8
```

**Tips for effective KPI descriptions**:
- Be specific about what counts. "Activities that improve team ability to ship quality software faster" scores code, tests, and CI/CD work well.
- The scorer compares each work title to each KPI description. Longer, more specific descriptions produce more accurate scores.
- Use `weight` (0.0–1.0) to express relative importance if KPIs aren't equally valued.
- There's no fixed list — define what your company actually measures. OKRs, board metrics, and department goals all work.

### Categories (the taxonomy)

Categories classify the *type* of work, independent of business value. Meridian ships with 10 built-in categories covering most teams:

| ID | Label | What it covers |
|----|-------|---------------|
| `code` | Code Implementation | Writing, debugging, refactoring, testing, reviewing code |
| `pm` | Project Management | Planning, roadmaps, sprints, tracking, retrospectives |
| `docs` | Documentation | Usage guides, READMEs, changelogs, internal docs |
| `security` | Security & Compliance | Reviews, hardening, audit, access control |
| `ops` | Operations & Infrastructure | Deployment, CI/CD, monitoring, devops |
| `finance` | Finance & Reporting | Budget analysis, cost tracking, financial modeling |
| `sales` | Sales & Business Development | Proposals, CRM, outreach, business development |
| `marketing` | Marketing & Content | Content, campaigns, brand, social |
| `product` | Product Management | Discovery, user research, requirements, UX decisions |
| `research` | Research & Exploration | Technical research, prototyping, POCs, investigation |

Each category has **subcategories** (L2) for finer-grained breakdown — for example, `code` breaks into: Core Feature Development, Data Modeling, Integration & Connectors, UI / Frontend, Testing & QA, Performance & Reliability, and Refactoring & Tech Debt.

**Most teams use the built-in categories.** The descriptions and keywords are tuned for software teams and general business work.

**To customize categories** — add `[[categories]]` blocks in `meridian.toml`. This *replaces* the full built-in set, so include all the categories you want:

```toml
# Custom taxonomy for a game studio — replaces the built-in set
[[categories]]
id = "gameplay"
label = "Gameplay & Mechanics"
description = "Game mechanics, player systems, physics, AI behavior, level design logic"
keywords = ["gameplay", "mechanic", "physics", "player", "level", "ai", "behavior"]

[[categories]]
id = "engine"
label = "Engine & Tools"
description = "Rendering, engine subsystems, editor tooling, build pipeline, performance"
keywords = ["engine", "renderer", "shader", "pipeline", "editor", "tool", "optimization"]

[[categories]]
id = "art_pipeline"
label = "Art Pipeline"
description = "Asset import, rig, animation, material authoring, texture, VFX"
keywords = ["art", "asset", "animation", "rig", "texture", "material", "vfx"]
```

**`domain_label` (subcategory override)**: Built-in subcategories have a `domain_label` field that lets you rename them for your vertical without rebuilding the keyword set. This is set in code (`taxonomy.rs`) for now; config-level overrides are planned.

### Business type presets

`meridian init --preset <type>` writes a ready-to-use `meridian.toml` tuned for your context. Pick the one closest to your team, then edit KPI descriptions to match your actual priorities.

```bash
meridian init --preset software      # default — engineering team, general software product
meridian init --preset saas          # SaaS product company (ARR, churn, product velocity)
meridian init --preset agency        # agency / service business (client delivery, creative quality)
meridian init --preset game-studio   # game development (player experience, art, audio, engine)
meridian init --preset enterprise    # large org (compliance, governance, risk, operational efficiency)
meridian init --preset startup       # early-stage (revenue, PMF, velocity, runway)
```

| Preset | KPIs | Categories |
|--------|------|-----------|
| `software` | Engineering Velocity, Revenue Growth, Risk Reduction, Customer Satisfaction | Built-in (code, pm, docs, security, ops…) |
| `saas` | Product Velocity, ARR Growth, Churn Reduction, Technical Health | Built-in |
| `agency` | Client Delivery, Revenue Growth, Creative Quality, Operational Efficiency | Custom: Client Work, Creative, Biz Dev, PM, Ops |
| `game-studio` | Player Experience, Feature Completion, Technical Performance, Revenue | Custom: Gameplay, Art, Audio, Engine, Production, Live Ops |
| `enterprise` | Compliance, Operational Efficiency, Risk Reduction, Stakeholder Value | Built-in |
| `startup` | Revenue Growth, Product-Market Fit, Engineering Velocity, Runway Extension | Built-in |

All presets include commented `[source]` stubs so you can switch between TA, JSONL, and Claude Code data with a single uncomment.

---

## Data sources

| Source key | What it reads | Effort unit |
|-----------|--------------|-------------|
| `ta` | `.ta/velocity-history.jsonl` in a TA project | seconds (wall clock) |
| `jsonl` | Any JSONL file you provide | tokens or seconds |
| `claude_code` | `~/.claude/projects/` session transcripts | tokens |

### In `meridian.toml`

```toml
[source]
# --- Pick one ---

# TA project: reads .ta/velocity-history.jsonl
# Auto-detected when running from a TA project directory.
ta_project_root = "/path/to/MyProject"

# Standalone JSONL: your own append-log of work records.
# Append one JSON object per line as work happens; use --since to scope reports.
# jsonl = "work.jsonl"

# Claude Code sessions: reads ~/.claude/projects/ for session history.
# Useful on any machine running Claude Code, no TA required.
# claude_code_dir = "~/.claude/projects"   # default path; omit to use default
```

Only one source is active at a time. If multiple are set, `ta_project_root` takes priority, then `jsonl`, then `claude_code`.

### JSONL format

Append one record per line. Meridian reads all records and filters by `--since` for time windows.

```jsonl
{"id": "task-001", "title": "Implement OAuth2 login flow", "tokens_input": 12000, "tokens_output": 4500, "timestamp": "2026-06-01T10:00:00Z"}
{"id": "task-002", "title": "Write Q2 board deck", "seconds": 7200, "timestamp": "2026-06-02T14:00:00Z"}
{"id": "task-003", "title": "Debug payment webhook failures", "tokens_input": 8000, "tokens_output": 2000}
```

- `title` is required. Everything else is optional.
- Use `tokens_input`/`tokens_output` for LLM token usage or `seconds` for wall-clock time. `timestamp` is RFC3339; defaults to record-load time if absent.
- Any LLM provider's usage log can be converted to this format with a small script.

**The append-log pattern**: Keep a single growing file (`work.jsonl`) and append records as work happens. Use `--since 7d` or `--since 2026-06-01` to scope reports to a time window. To archive a period, rename the file (e.g. `work-2026-q2.jsonl`) and start a new one.

> **Directory scanning** (planned): Meridian will support `jsonl = "logs/"` to scan a directory for `*.jsonl` files. Until then: `cat logs/*.jsonl | meridian analyze --source jsonl --path /dev/stdin`.

---

## Configuration reference

```bash
meridian init    # creates meridian.toml in current directory
```

```toml
# meridian.toml

[source]
# ta_project_root = "/path/to/TrustedAutonomy"   # or auto-detected from cwd
# jsonl = "work.jsonl"                           # standalone append-log
# claude_code_dir = "~/.claude/projects"         # Claude Code session history

[report]
format = "table"    # table | json | csv

# --- Define your KPIs ---
# Descriptions drive scoring accuracy. Be specific about what counts.

[[kpis]]
id = "engineering_velocity"
label = "Engineering Velocity"
description = "Activities that improve team ability to ship quality software faster"
weight = 1.0

[[kpis]]
id = "revenue_growth"
label = "Revenue Growth"
description = "Activities that drive or support revenue — sales, features, marketing"
weight = 1.0

# --- Custom categories (optional) ---
# Omit this section to use the 10 built-in categories (recommended for most teams).
# If you add any [[categories]] block, it replaces the full built-in set.
#
# [[categories]]
# id = "gameplay"
# label = "Gameplay & Mechanics"
# description = "Game mechanics, player systems, physics, AI, level design"
# keywords = ["gameplay", "mechanic", "physics", "player", "level"]

# --- Embedding backend (optional, improves accuracy on short/ambiguous titles) ---
# Default "keyword" scorer works offline with zero API calls.
# [embedding]
# backend = "voyage"     # requires VOYAGE_API_KEY
# model = "voyage-3-lite"

# --- Regression suggestions (optional, requires ANTHROPIC_API_KEY) ---
# [suggest]
# threshold = 0.25       # categories below 25% alignment get suggestions
# model = "claude-haiku-4-5-20251001"
```

---

## Commands

```bash
meridian analyze                     # classify + report (auto-detects source)
meridian analyze --source jsonl --path work.jsonl
meridian analyze --since 7d          # last 7 days only
meridian analyze --since 2026-06-01  # since a specific date
meridian analyze --format json       # JSON output
meridian analyze --format csv        # CSV output

meridian suggest                     # generate alignment suggestions (requires ANTHROPIC_API_KEY)
meridian suggest --dry-run           # show which pairs would get suggestions, no API call
meridian suggest --threshold 0.4     # custom threshold

meridian summarize-title --text "..."  # extract a clean work title from raw prompt text
echo "some prompt" | meridian summarize-title

meridian serve                       # start MCP server (stdio transport)
meridian init                        # create meridian.toml (default: --preset software)
meridian init --preset saas          # SaaS KPIs (ARR growth, churn, product velocity)
meridian init --preset agency        # agency KPIs + custom categories
meridian init --preset game-studio   # game studio KPIs + gameplay/art/audio/engine categories
meridian init --preset enterprise    # compliance, governance, risk KPIs
meridian init --preset startup       # revenue, PMF, velocity, runway KPIs
```

## Embedding backend upgrade

The default keyword scorer works offline with zero cost and is a good starting point. For teams with short or cryptic work titles, switching to semantic embeddings improves classification accuracy significantly.

```bash
# Get a Voyage AI API key: https://www.voyageai.com
export VOYAGE_API_KEY=your_key_here
```

```toml
# meridian.toml
[embedding]
backend = "voyage"
model = "voyage-3-lite"   # cheapest; use voyage-3 for higher accuracy
```

Voyage AI embeds categories and KPIs once at startup, then classifies each record with a single cosine similarity lookup — minimal latency, ~$0.00002 per 1K tokens.

## Scoring accuracy: keyword vs. embedding

| Approach | Accuracy (typical) | Cost | Offline? |
|----------|-------------------|------|---------|
| Keyword TF-IDF (default) | ~75% for clear titles | Free | Yes |
| Voyage AI embedding | ~90%+ | ~$0.00002/1K tokens | No |

Use keyword scoring to get started; switch to embeddings when you need higher fidelity or are classifying short/ambiguous titles like `"v0.17.0.10 - Generic Plugin System"`.

## MCP server reference

Meridian exposes its analytics as an MCP server compatible with Claude Code, Claude Desktop, and any MCP-capable agent framework.

```bash
# Register with Claude Code (stdio transport — managed by the MCP host)
claude mcp add meridian -- meridian serve

# With an explicit config path
claude mcp add meridian -- meridian serve --config /path/to/meridian.toml
```

### Available tools

| Tool | Parameters | Returns |
|------|------------|---------|
| `meridian_report` | `since` (default: 7d), `source`, `path` | Spend table + KPI scores by session |
| `meridian_analyze` | `source`, `path` | Category breakdown with effort % and KPI alignment |
| `meridian_kpis` | — | Configured KPIs with weights, descriptions, metrics |
| `meridian_suggest` | `source`, `path`, `threshold` | Low-alignment pairs flagged for rebalancing |
| `meridian_summarize_title` | `text` | Derived work title (≤8 words) stripped of injected context |

### Publishing to MCP marketplaces

- **Smithery** (`mcp.smithery.ai`): PR a `smithery.yaml` into `smithery-ai/registry`
- **Anthropic MCP list**: PR into `modelcontextprotocol/servers`
- **npm wrapper**: Publish `meridian` to npm with a platform-specific binary download for zero-install `npx meridian serve`

## TA integration

Meridian reads Trusted Autonomy's `.ta/velocity-history.jsonl` directly — no changes to TA required.

```bash
# From any TA project root:
meridian analyze

# Or with explicit path:
meridian analyze --source ta --path ~/development/TrustedAutonomy

# Via the ta CLI (v0.17.0.12+):
ta meridian report
ta meridian analyze
```

## Architecture

```
meridian-core      — UsageRecord, Taxonomy, KeywordScorer, EmbeddingScorer, Embedder trait
meridian-ingest    — TA adapter (velocity-history.jsonl), generic JSONL, Claude Code sessions
meridian-report    — table/JSON/CSV output, regression suggestion engine
meridian-config    — meridian.toml loader
apps/meridian      — CLI binary (analyze, suggest, init, serve)
```

## License

MIT
