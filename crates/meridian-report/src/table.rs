use crate::summary::AnalysisSummary;
use std::collections::HashMap;

pub fn print_summary(summary: &AnalysisSummary, category_labels: &HashMap<String, String>) {
    println!("\nMeridian — Effort Analysis");
    println!("{}", "\u{2500}".repeat(72));
    println!(
        "{:<24} {:>8} {:>12} {:>8}",
        "Category", "Records", "Effort", "% Total"
    );
    println!("{}", "\u{2500}".repeat(72));
    for cat in &summary.by_category {
        let label = category_labels
            .get(&cat.category_id)
            .cloned()
            .unwrap_or_else(|| cat.category_id.clone());
        println!(
            "{:<24} {:>8} {:>12.0} {:>7.1}%",
            label, cat.record_count, cat.total_effort, cat.effort_pct
        );
        if !cat.kpi_alignment.is_empty() {
            let mut kpi_pairs: Vec<(&String, &f32)> = cat.kpi_alignment.iter().collect();
            kpi_pairs.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            let kpi_str: Vec<String> = kpi_pairs
                .iter()
                .map(|(id, score)| format!("{}:{:.0}%", id, **score * 100.0))
                .collect();
            println!("  KPI: {}", kpi_str.join("  "));
        }
    }
    println!("{}", "\u{2500}".repeat(72));
    println!(
        "Total: {} records, {:.0} effort points",
        summary.total_records, summary.total_effort
    );
}
