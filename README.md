# Meridian

Token spend and KPI alignment analytics for AI agent workflows.

Meridian answers: **Where is your team's AI effort going, and does it align with what matters?**

It classifies agent work by business category (code, PM, docs, sales, etc.), scores each category against your company KPIs, and — optionally — generates concrete suggestions for improving alignment.

---

**Available as:**
- **MCP tool** — add to any Claude session with one command; your agent can query spend, analyze alignment, and get suggestions without leaving the conversation
- **Trusted Autonomy integration** — built-in via `ta meridian`; every agent goal run is automatically tracked and available for KPI analysis
- **Standalone CLI** — works with any JSONL usage log, TA project directory, or Claude Code projects folder

---

## Quick start

### As an MCP tool (Claude Code / Claude Desktop)

```bash
# Install
cargo install --git https://github.com/Trusted-Autonomy/meridian

# Add to Claude Code — runs meridian serve as a local MCP server
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

### Standalone CLI

```bash
cargo install --git https://github.com/Trusted-Autonomy/meridian

# From a TA project directory (auto-detected):
meridian analyze

# Standalone — any work log in JSONL format:
meridian init                                     # creates meridian.toml with example KPIs
meridian analyze --source jsonl --path work.jsonl
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

## Data sources

| Source | What it reads | Effort unit |
|--------|--------------|-------------|
| `--source ta` | `.ta/velocity-history.jsonl` | seconds (wall clock) |
| `--source jsonl` | Any JSONL with `{title, id?, tokens_input?, tokens_output?, seconds?}` | tokens or seconds |

### Standalone JSONL format

```jsonl
{"id": "task-001", "title": "Implement OAuth2 login flow", "tokens_input": 12000, "tokens_output": 4500, "timestamp": "2026-06-01T10:00:00Z"}
{"id": "task-002", "title": "Write Q2 board deck", "seconds": 7200, "timestamp": "2026-06-02T14:00:00Z"}
{"id": "task-003", "title": "Debug payment webhook failures", "tokens_input": 8000, "tokens_output": 2000}
```

Any LLM provider's usage log can be transformed into this format with a simple script. Fields `tokens_input`/`tokens_output` and `seconds` are alternatives — use whichever your source provides.

## Configuration

```bash
meridian init    # creates meridian.toml in current directory
```

```toml
# meridian.toml

[source]
ta_project_root = "/path/to/TrustedAutonomy"   # or auto-detected from cwd
# jsonl = "my-usage.jsonl"                     # standalone

[report]
format = "table"    # table | json | csv

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

# Optional: semantic embedding backend (improves accuracy on short titles)
# [embedding]
# backend = "voyage"     # requires VOYAGE_API_KEY
# model = "voyage-3-lite"

# Optional: regression suggestions via Claude
# [suggest]
# threshold = 0.25       # categories below 25% alignment get suggestions
# model = "claude-haiku-4-5-20251001"
# # Reads ANTHROPIC_API_KEY from env
```

## Commands

```bash
meridian analyze                     # classify + report (auto-detects source)
meridian analyze --format json       # JSON output
meridian analyze --format csv        # CSV output

meridian suggest                     # generate alignment suggestions (requires ANTHROPIC_API_KEY)
meridian suggest --dry-run           # show which pairs would get suggestions, no API call
meridian suggest --threshold 0.4    # custom threshold

meridian summarize-title --text "..."  # extract a clean work title from raw prompt text
echo "some prompt" | meridian summarize-title   # reads from stdin

meridian serve                       # start MCP server (stdio transport)
meridian init                        # create meridian.toml
```

## Sample output

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
meridian-ingest    — TA adapter (velocity-history.jsonl), generic JSONL, Anthropic CSV stub
meridian-report    — table/JSON/CSV output, regression suggestion engine
meridian-config    — meridian.toml loader
apps/meridian      — CLI binary (analyze, suggest, init)
```

## License

MIT
