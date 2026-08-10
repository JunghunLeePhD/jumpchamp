// ============================================================================
// Pure Reporting & Formatting
// ============================================================================

use std::collections::BTreeMap;
use super::gaps::RecordGap;

/// Formats a gap frequency map as an aligned text report, showing entries between `top_min` and `top_max` rank.
pub fn format_report(freq_map: &BTreeMap<u64, u64>, top_min: usize, top_max: usize) -> String {
    let total_pairs: u64 = freq_map.values().sum();
    let mut sorted: Vec<_> = freq_map.iter().map(|(&diff, &count)| (diff, count)).collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut out = String::new();
    out.push_str(&format!("{:<12} {:<15} {:<12}\n", "Diff", "Frequency", "Percentage"));
    out.push_str(&format!("{}\n", "-".repeat(42)));

    let start_idx = (top_min.saturating_sub(1)).min(sorted.len());
    let end_idx = top_max.min(sorted.len()).max(start_idx);

    for (diff, count) in &sorted[start_idx..end_idx] {
        let pct = (*count as f64 / total_pairs as f64) * 100.0;
        out.push_str(&format!("{:<12} {:<15} {:.2}%\n", diff, count, pct));
    }

    out.push_str(&format!("{}\n", "-".repeat(42)));
    out.push_str(&format!("Total Analyzed Pairs: {}\n", total_pairs));
    out
}

/// Formats a list of record-breaking prime gaps into a text table with Cramér Ratios.
pub fn format_record_gaps_report(records: &[RecordGap]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{:<12} {:<16} {:<12} {:<15}\n",
        "Index (n)", "Prime (p_n)", "Gap (Δ)", "Cramér Ratio"
    ));
    out.push_str(&format!("{}\n", "-".repeat(58)));

    for r in records {
        out.push_str(&format!(
            "{:<12} {:<16} {:<12} {:.4}\n",
            r.prime_index, r.prime, r.gap, r.cramer_ratio
        ));
    }
    out.push_str(&format!("{}\n", "-".repeat(58)));
    out.push_str(&format!("Total Record Gaps Found: {}\n", records.len()));
    out
}

/// Formats residue class distributions (e.g. g mod 6) as an aligned report.
pub fn format_residue_report(residues: &BTreeMap<u64, u64>, modulus: u64) -> String {
    let total: u64 = residues.values().sum();
    let mut out = String::new();
    out.push_str(&format!(
        "{:<15} {:<15} {:<12}\n",
        format!("Residue (mod {})", modulus), "Frequency", "Percentage"
    ));
    out.push_str(&format!("{}\n", "-".repeat(45)));

    for (&rem, &count) in residues {
        let pct = (count as f64 / total as f64) * 100.0;
        out.push_str(&format!("{:<15} {:<15} {:.2}%\n", rem, count, pct));
    }
    out.push_str(&format!("{}\n", "-".repeat(45)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_report() {
        let mut map = BTreeMap::new();
        map.insert(2, 50);
        map.insert(4, 30);
        map.insert(6, 20);

        let report = format_report(&map, 1, 2);
        assert!(report.contains("Diff"));
        assert!(report.contains("Total Analyzed Pairs: 100"));
    }
}

