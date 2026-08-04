# Per-Entry KPI Alignment Tiers — Design Spec

**Status**: Approved, building now
**Scope**: Meridian only. TA-side ingestion mapping (goal-audit.jsonl → `prompt`/`objective`/`credits_consumed`) is explicitly a later, separate phase in the TA repo — not built here. Meridian must not gain any TA-specific knowledge.

## 1. Problem

`KeywordScorer::classify` already computes a per-record score against every configured KPI (`crates/meridian-core/src/scorer.rs`), but that detail is discarded immediately after being averaged into category-level summaries (`crates/meridian-report/src/summary.rs`). There is no way today to see which *individual* work items are poorly aligned with all tracked KPIs, or to get aggregate stats (count/effort/cost) split by how well-aligned the work actually is. A leader reviewing spend has no way to answer "show me the specific work that isn't contributing to anything we track."

## 2. Data Model Changes

`UsageRecord` (`crates/meridian-core/src/record.rs`) gains three new optional, source-agnostic fields. Meridian stays generic — these are simply richer optional context a source *may* supply, not TA-specific:

```rust
pub struct UsageRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub effort: EffortUnits,
    pub source: String,
    pub phase: Option<String>,
    pub metadata: HashMap<String, String>,
    pub prompt: Option<String>,        // NEW: fuller task description, if the source has one
    pub objective: Option<String>,     // NEW: stated goal/intent, if the source has one
    pub credits_consumed: Option<f64>, // NEW: generic cost unit; source defines meaning
}
```

Existing ingestion sources (`meridian-ingest`'s `ta`, `claude_code`, `generic`) are updated only to set these three fields to `None` explicitly at construction (no behavior change — they don't populate them; that's the deferred TA-integration phase's job).

## 3. Tiering Logic

New function in `crates/meridian-report`, alongside `summarise`: for each `ClassifiedRecord`, compute `best_kpi_score = kpi_scores.iter().map(|s| s.score).fold(f32::MIN, f32::max)` (0.0 if no KPIs configured — an explicit empty-KPI-list case, not a panic). Bucket into one of three tiers via two configurable thresholds:

```rust
pub struct AlignmentConfig {
    pub low_threshold: f32,   // default 0.25 — below this: Low
    pub high_threshold: f32,  // default 0.6  — at/above this: High
    // between the two: Moderate
}
```

New `[alignment]` section in `meridian.toml`, following the exact pattern already used by `[suggest]`:

```toml
[alignment]
low_threshold = 0.25
high_threshold = 0.6
```

Per tier, aggregate: entry count, sum of `effort.effort_points()`, sum of `credits_consumed` (where `Some`; report `—` not `0` when no entry in a tier has a value, so "not provided" is visibly distinct from "zero cost"), and average `best_kpi_score`.

## 4. CLI Surface

Extends `meridian report` (`apps/meridian/src/commands/report.rs`) — no new command. New `ReportArgs` fields:

```rust
/// Show individual matching records. Bare = all; with a number = top N by score (ascending).
#[arg(long, num_args = 0..=1, default_missing_value = "18446744073709551615")]
pub detail: Option<usize>,   // None = not passed; Some(usize::MAX) = bare; Some(n) = limit

/// Restrict to one named tier: high, moderate, low. Implies --detail if not otherwise set.
#[arg(long, conflicts_with_all = ["min_score", "max_score"])]
pub tier: Option<String>,

/// Restrict to entries scoring at or above this value. Implies --detail.
#[arg(long)]
pub min_score: Option<f32>,

/// Restrict to entries scoring at or below this value. Implies --detail.
#[arg(long)]
pub max_score: Option<f32>,
```

**Behavior**:
- No flags: prints only the tier-summary table (counts/effort/credits/avg score per tier), exactly as today's `report` output shape, in whatever `--format` is selected (table/json/csv — unchanged).
- `--detail` (bare) or any of `--tier`/`--min-score`/`--max-score`: summary table, then every matching entry — title, best-fit KPI id, score, `prompt`/`objective` if present (omit those fields entirely in table view if all matching entries lack them; always include as `null`/empty in json/csv for consistent schema).
- `--detail N`: same as above, capped to the top N matching entries.
- **Sort order**: detail output is always ascending by `best_kpi_score` (worst-first) — even within the High tier, so a leader scanning any tier sees the weakest/most-borderline entries first.
- `--tier` and `--min-score`/`--max-score` are mutually exclusive (`conflicts_with_all`, a clean clap-level error, not a runtime check).
- Tier name matching for `--tier` is case-insensitive (`high`/`High`/`HIGH` all valid); invalid tier name is a clear error listing the three valid values.

**Example**:
```
$ meridian report --since 30d --min-score 0.25 --max-score 0.35

KPI Alignment Summary (since 30d)
─────────────────────────────────────────────
Tier       Count   Effort      Credits   Avg Score
High       12      4h 20m      —         0.74
Moderate   31      11h 05m     —         0.41
Low        18      6h 40m      —         0.11

Matching records (score 0.25–0.35): 9
─────────────────────────────────────────────
[0.27] "Fix flaky test in daemon watchdog" — best fit: engineering_velocity
[0.29] "Rotate staging credentials"         — best fit: risk_reduction
...
```

## 5. Explicitly Out of Scope (this phase)

- Any visual/HTML/chart export — dropped per direct instruction; stays table/json/csv only.
- TA→Meridian mapping of `goal-audit.jsonl` into `prompt`/`objective`/`credits_consumed` — belongs to the later "update TA to use it" phase, in the TA repo.
- Creating real KPI definitions for TA and running v0.17's own phases through this report — a valuable validation exercise, explicitly noted as an immediate next step once this ships, not part of building the feature itself.

## 6. Testing

- Tier-boundary edge cases: score exactly `0.25`, exactly `0.6` (boundary inclusive on the high side per `>= high_threshold`, exclusive on the low side per `< low_threshold`).
- Aggregate math: count/effort/credits sums per tier against a hand-built fixture with known values.
- `--detail`, `--detail N`, `--tier`, `--min-score`/`--max-score` filtering — each produces the expected subset, correctly sorted ascending.
- `--tier` + `--min-score` together produces a clean clap error, not a panic or silent override.
- Unmapped `prompt`/`objective`/`credits_consumed` (existing sources, all `None`) render cleanly in every output format — no `Some(None)` artifacts, no panics, `—` shown for missing credits rather than `0`.
- Empty-KPI-config case: `best_kpi_score` defaults sensibly (0.0, tier = Low) rather than panicking on an empty iterator fold.
