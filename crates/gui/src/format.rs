//! Number formatting for labels.

/// Compact form: `1.50 M`, `12.3 K`, `999`.
pub fn compact(val: u64) -> String {
    const UNITS: [(u64, f64, &str); 4] = [
        (1_000_000_000_000, 1e12, "T"),
        (1_000_000_000, 1e9, "B"),
        (1_000_000, 1e6, "M"),
        (1_000, 1e3, "K"),
    ];
    match UNITS.iter().find(|(min, ..)| val >= *min) {
        Some((1_000, div, unit)) => format!("{:.1} {unit}", val as f64 / div),
        Some((_, div, unit)) => format!("{:.2} {unit}", val as f64 / div),
        None => val.to_string(),
    }
}

/// Digit grouping: `1234567` → `1,234,567`.
pub fn thousands(val: u64) -> String {
    let digits = val.to_string();
    let len = digits.len();
    digits
        .chars()
        .enumerate()
        .flat_map(|(i, ch)| {
            (i > 0 && (len - i).is_multiple_of(3))
                .then_some(',')
                .into_iter()
                .chain([ch])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact() {
        assert_eq!(compact(999), "999");
        assert_eq!(compact(12_345), "12.3 K");
        assert_eq!(compact(10_000_000), "10.00 M");
        assert_eq!(compact(2_500_000_000), "2.50 B");
        assert_eq!(compact(3_000_000_000_000), "3.00 T");
    }

    #[test]
    fn test_thousands() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }
}
