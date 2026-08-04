# Meridian — Claude Code Project Instructions

## Build
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Design
- **meridian-core**: `UsageRecord` (title + optional `prompt`/`objective`/`credits_consumed`, source-agnostic), `Taxonomy`, `KeywordScorer` (bi-prism TF-IDF cosine)
- **meridian-ingest**: source adapters (TA velocity-history.jsonl, generic JSONL, Anthropic export stub)
- **meridian-report**: table/JSON/CSV output; `alignment` module for per-entry KPI-fit tiering
- **meridian-config**: `MeridianConfig` loaded from `meridian.toml`, incl. `AlignmentConfig`
- **apps/meridian**: CLI binary (`meridian analyze`, `meridian report`, `meridian init`)

## TA Integration
Reads `.ta/velocity-history.jsonl` (time-based effort records). Auto-discovered from cwd walking up.
Token counts come from generic JSONL or future TA token tracking.
TA does not populate `prompt`/`objective`/`credits_consumed` yet — that mapping (e.g. from
`goal-audit.jsonl`) is separate, TA-side work; Meridian itself stays source-agnostic and never
assumes TA-specific shapes.

## Standalone
Works without TA: `meridian analyze --source jsonl --path records.jsonl`
Input format: `{"title": "...", "id"?: "...", "tokens_input"?: N, "tokens_output"?: N, "seconds"?: N}`

## KPI Alignment Tiers
`meridian report` always shows a High/Moderate/Low tier summary (count, effort, credits, avg
score) based on each record's best-fit KPI score. Thresholds are configurable via `[alignment]`
in `meridian.toml` (defaults: `low_threshold = 0.25`, `high_threshold = 0.6`). Drill into
individual records with `--detail [N]` (bare = all, `--detail 20` = top 20 by score ascending),
`--tier <high|moderate|low>`, or `--min-score`/`--max-score` (mutually exclusive with `--tier`).
See `docs/superpowers/specs/2026-07-25-kpi-alignment-tiers-design.md` for the full design.
