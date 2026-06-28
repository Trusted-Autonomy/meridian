use anyhow::Result;
use meridian_core::record::UsageRecord;
use std::path::Path;

/// Load from Anthropic console usage export (CSV).
/// Not yet implemented — use generic JSONL in the meantime.
pub fn load_csv(_path: &Path) -> Result<Vec<UsageRecord>> {
    anyhow::bail!(
        "Anthropic CSV import is not yet implemented.\n\
         Export your usage as JSONL and use --source jsonl instead."
    )
}
