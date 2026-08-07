// ============================================================================
// Pure Reporting & Formatting
// ============================================================================

use std::collections::BTreeMap;

/// Formats a gap frequency map as an aligned text report, showing the top `top_n` entries.
pub fn format_report(freq_map: &BTreeMap<u64, u64>, top_n: usize) -> String {
    let total_pairs: u64 = freq_map.values().sum();
    let mut sorted: Vec<_> = freq_map.iter().map(|(&diff, &count)| (diff, count)).collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = String::new();
    out.push_str(&format!("{:<12} {:<15} {:<12}\n", "Diff", "Frequency", "Percentage"));
    out.push_str(&format!("{}\n", "-".repeat(42)));

    for (diff, count) in sorted.into_iter().take(top_n) {
        let pct = (count as f64 / total_pairs as f64) * 100.0;
        out.push_str(&format!("{:<12} {:<15} {:.2}%\n", diff, count, pct));
    }

    out.push_str(&format!("{}\n", "-".repeat(42)));
    out.push_str(&format!("Total Analyzed Pairs: {}\n", total_pairs));
    out
}
