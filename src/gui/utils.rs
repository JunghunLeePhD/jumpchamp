// ============================================================================
// Shared GUI Utilities & Formatter Helpers
// ============================================================================

pub fn format_compact_num(val: u64) -> String {
    if val >= 1_000_000_000_000 {
        format!("{:.2} T", val as f64 / 1e12)
    } else if val >= 1_000_000_000 {
        format!("{:.2} B", val as f64 / 1e9)
    } else if val >= 1_000_000 {
        format!("{:.2} M", val as f64 / 1e6)
    } else if val >= 1_000 {
        format!("{:.1} K", val as f64 / 1e3)
    } else {
        format!("{}", val)
    }
}

pub fn format_thousands(val: u64) -> String {
    let s = val.to_string();
    let mut result = String::new();
    let len = s.len();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}
