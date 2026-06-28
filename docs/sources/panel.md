# Expert Panel Scoring & Weekly Reports

`meridian report` evaluates your sessions against a panel of virtual executives, producing a per-role breakdown, consensus score, and dissent signal — no API calls required.

## How it works

Each panelist has a **charter**: a description of what they care about. Meridian scores every session title against each charter using TF-IDF cosine similarity (the same zero-cost engine as `meridian analyze`).

The report shows:
- **Per-role score** (0.0–1.0) for each panelist
- **Consensus** — weighted average across the panel
- **Dissent** — standard deviation; high values mean panelists disagree
- **Top KPI** — highest-scoring KPI from your `[[kpis]]` config (if defined)
- **High-dissent callouts** — sessions where panelists disagree by more than 0.20

### Built-in panel (default)

Used automatically when no `[[panel]]` is configured:

| Role | Charter focus |
|---|---|
| `ceo` | Revenue growth, strategic alignment, ROI |
| `cto` | Technical debt, reliability, security, architecture |
| `head_of_product` | User value, feature velocity, roadmap coherence |
| `head_of_engineering` | Code quality, test coverage, delivery predictability |

## Quick start

```bash
# Last 7 days (default)
meridian report

# Last 30 days
meridian report --since 30d

# Since a specific date
meridian report --since 2026-06-01

# Last 2 weeks
meridian report --since 2w

# Machine-readable output
meridian report --format json
meridian report --format csv
```

## Customizing the panel

Add `[[panel]]` sections to `meridian.toml`. If any panels are configured, the built-in defaults are replaced entirely.

```toml
[[panel]]
role = "cpo"
charter = "Product vision, market timing, customer discovery, monetization strategy"
weight = 1.2

[[panel]]
role = "vp_engineering"
charter = "Engineering execution, hiring, technical standards, team health"
weight = 1.0

[[panel]]
role = "cfo"
charter = "Budget efficiency, ROI, cost reduction, financial risk management"
weight = 0.8
```

`weight` affects the consensus score (higher-weight panelists count more in the average). It does not affect dissent calculation — dissent measures raw disagreement.

## Interpreting the output

```
Report  2026-06-21 → 2026-06-28   (12 sessions)

 Session                              CEO   CTO  PROD   ENG  Consensus  Dissent  Top KPI
 ─────────────────────────────────────────────────────────────────────────────────────────
 Fix auth token expiry bug           0.15  0.82  0.31  0.88      0.53     0.32!  tech_debt
 Weekly planning sync                0.44  0.28  0.51  0.30      0.38     0.09  product
 ...
 ─────────────────────────────────────────────────────────────────────────────────────────
 Period average                      0.32  0.55  0.40  0.61      0.47     0.14

 High dissent (>0.20):
   "Fix auth token expiry bug" — cto 0.82 vs ceo 0.15
```

- `!` suffix on Dissent means the session exceeded the 0.20 dissent threshold
- High-consensus, low-dissent work is well-understood across all perspectives
- High-dissent work often represents domain-specific depth (engineering-heavy, product-heavy) — worth a one-line call-out in retrospectives

## Scheduled weekly reports

### Cron (Linux/macOS)

```cron
0 9 * * MON  cd /path/to/project && meridian report --since 7d >> reports/weekly.txt
```

### GitHub Actions

```yaml
name: Weekly Meridian Report
on:
  schedule:
    - cron: '0 9 * * 1'   # 9am UTC every Monday
  workflow_dispatch:

jobs:
  report:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install meridian
      - run: meridian report --since 7d --source jsonl --path records.jsonl
        env:
          # Optional: set if your records.jsonl has a path configured
          MERIDIAN_CONFIG: meridian.toml
```

## Difference from `meridian analyze`

| | `meridian analyze` | `meridian report` |
|---|---|---|
| **Scope** | All records | Time-windowed (default: 7d) |
| **Output** | Category breakdown + KPI table | Per-panelist scores + dissent |
| **API cost** | Zero | Zero |
| **Purpose** | What categories am I spending time on? | How does each executive perspective rate my work? |

Both commands use the same source configuration — they read from the same TA/Claude Code/JSONL data.
