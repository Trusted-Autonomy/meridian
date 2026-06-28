# Meridian — Claude Code Project Instructions

## Build
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Design
- **meridian-core**: `UsageRecord`, `Taxonomy`, `KeywordScorer` (bi-prism TF-IDF cosine)
- **meridian-ingest**: source adapters (TA velocity-history.jsonl, generic JSONL, Anthropic export stub)
- **meridian-report**: table/JSON/CSV output
- **meridian-config**: `MeridianConfig` loaded from `meridian.toml`
- **apps/meridian**: CLI binary (`meridian analyze`, `meridian init`)

## TA Integration
Reads `.ta/velocity-history.jsonl` (time-based effort records). Auto-discovered from cwd walking up.
Token counts come from generic JSONL or future TA token tracking.

## Standalone
Works without TA: `meridian analyze --source jsonl --path records.jsonl`
Input format: `{"title": "...", "id"?: "...", "tokens_input"?: N, "tokens_output"?: N, "seconds"?: N}`
