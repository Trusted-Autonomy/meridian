use crate::summary::AnalysisSummary;
use anyhow::Result;
use meridian_core::result::ClassifiedRecord;

pub fn to_json(summary: &AnalysisSummary) -> Result<String> {
    Ok(serde_json::to_string_pretty(summary)?)
}

pub fn to_csv(records: &[ClassifiedRecord]) -> String {
    let mut out = String::from("id,timestamp,title,source,category,confidence,effort_points\n");
    for r in records {
        let title = r.record.title.replace('"', "'");
        out.push_str(&format!(
            "{},{},\"{}\",{},{},{:.3},{:.1}\n",
            r.record.id,
            r.record.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
            title,
            r.record.source,
            r.category_id,
            r.category_confidence,
            r.record.effort.effort_points(),
        ));
    }
    out
}
