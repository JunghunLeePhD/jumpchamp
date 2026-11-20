// ============================================================================
// Pure Prime Factorization & Primorial (p_n#) Analytics
// ============================================================================

/// Represents a primorial `p_n# = 2 · 3 · 5 · ... · p_n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Primorial {
    pub index: usize,
    pub prime: u64,
    pub label: &'static str,
    pub value: u64,
}

/// All primorials within `MAX_GAP = 65_536`.
pub const PRIMORIALS: [Primorial; 6] = [
    Primorial {
        index: 1,
        prime: 2,
        label: "2#",
        value: 2,
    },
    Primorial {
        index: 2,
        prime: 3,
        label: "3#",
        value: 6,
    },
    Primorial {
        index: 3,
        prime: 5,
        label: "5#",
        value: 30,
    },
    Primorial {
        index: 4,
        prime: 7,
        label: "7#",
        value: 210,
    },
    Primorial {
        index: 5,
        prime: 11,
        label: "11#",
        value: 2_310,
    },
    Primorial {
        index: 6,
        prime: 13,
        label: "13#",
        value: 30_030,
    },
];

/// Returns the static list of primorials covered by the gap engine.
pub fn primorials() -> &'static [Primorial] {
    &PRIMORIALS
}

/// Computes the prime factorization of `n` as ascending `(prime, exponent)` pairs.
///
/// Returns an empty vector for `0` and `1`.
pub fn prime_factors(mut n: u64) -> Vec<(u64, u32)> {
    if n <= 1 {
        return Vec::new();
    }

    let mut factors = Vec::new();
    if n.is_multiple_of(2) {
        let exp = n.trailing_zeros();
        n >>= exp;
        factors.push((2, exp));
    }

    let mut d = 3u64;
    while d <= n / d {
        if n.is_multiple_of(d) {
            let mut exp = 0u32;
            while n.is_multiple_of(d) {
                n /= d;
                exp += 1;
            }
            factors.push((d, exp));
        }
        d += 2;
    }

    if n > 1 {
        factors.push((n, 1));
    }

    factors
}

/// Formats the prime factorization of `n` (e.g. `12 -> "2^2 · 3"`).
pub fn format_prime_factorization(n: u64) -> String {
    match n {
        0 => "0".to_string(),
        1 => "1".to_string(),
        _ => prime_factors(n)
            .into_iter()
            .map(|(p, exp)| {
                if exp == 1 {
                    p.to_string()
                } else {
                    format!("{p}^{exp}")
                }
            })
            .collect::<Vec<_>>()
            .join(" · "),
    }
}

/// Returns the matching [`Primorial`] if `n` equals `p_k#` for some `k`.
pub fn is_primorial(n: u64) -> Option<Primorial> {
    PRIMORIALS.iter().copied().find(|p| p.value == n)
}

/// Returns the largest primorial in [`PRIMORIALS`] that evenly divides `n`.
///
/// Returns `None` if `n == 0` or `n` is odd.
pub fn highest_primorial_divisor(n: u64) -> Option<Primorial> {
    if n == 0 || !n.is_multiple_of(2) {
        return None;
    }
    PRIMORIALS
        .iter()
        .rev()
        .copied()
        .find(|p| n.is_multiple_of(p.value))
}

/// Formats `n` in terms of its highest primorial divisor (e.g. `6 -> "3#"`, `12 -> "2·3#"`).
pub fn format_primorial_multiple(n: u64) -> Option<String> {
    let p = highest_primorial_divisor(n)?;
    let q = n / p.value;
    if q == 1 {
        Some(p.label.to_string())
    } else {
        Some(format!("{q}·{}", p.label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prime_factors_edge_cases_and_composites() {
        assert_eq!(prime_factors(0), Vec::<(u64, u32)>::new());
        assert_eq!(prime_factors(1), Vec::<(u64, u32)>::new());
        assert_eq!(prime_factors(2), vec![(2, 1)]);
        assert_eq!(prime_factors(4), vec![(2, 2)]);
        assert_eq!(prime_factors(6), vec![(2, 1), (3, 1)]);
        assert_eq!(prime_factors(12), vec![(2, 2), (3, 1)]);
        assert_eq!(prime_factors(30), vec![(2, 1), (3, 1), (5, 1)]);
        assert_eq!(prime_factors(210), vec![(2, 1), (3, 1), (5, 1), (7, 1)]);
        assert_eq!(prime_factors(97), vec![(97, 1)]);
    }

    #[test]
    fn test_format_prime_factorization() {
        assert_eq!(format_prime_factorization(0), "0");
        assert_eq!(format_prime_factorization(1), "1");
        assert_eq!(format_prime_factorization(2), "2");
        assert_eq!(format_prime_factorization(4), "2^2");
        assert_eq!(format_prime_factorization(6), "2 · 3");
        assert_eq!(format_prime_factorization(12), "2^2 · 3");
        assert_eq!(format_prime_factorization(30), "2 · 3 · 5");
        assert_eq!(format_prime_factorization(210), "2 · 3 · 5 · 7");
    }

    #[test]
    fn test_primorials_and_exact_lookup() {
        let list = primorials();
        assert_eq!(list.len(), 6);
        assert_eq!(list[0].label, "2#");
        assert_eq!(list[0].value, 2);
        assert_eq!(list[5].label, "13#");
        assert_eq!(list[5].value, 30_030);

        assert_eq!(is_primorial(6).map(|p| p.label), Some("3#"));
        assert_eq!(is_primorial(30).map(|p| p.label), Some("5#"));
        assert_eq!(is_primorial(12), None);
        assert_eq!(is_primorial(0), None);
    }

    #[test]
    fn test_highest_primorial_divisor_and_multiple_formatting() {
        assert_eq!(highest_primorial_divisor(0), None);
        assert_eq!(highest_primorial_divisor(1), None);
        assert_eq!(highest_primorial_divisor(15), None);
        assert_eq!(highest_primorial_divisor(4).map(|p| p.label), Some("2#"));
        assert_eq!(highest_primorial_divisor(12).map(|p| p.label), Some("3#"));
        assert_eq!(highest_primorial_divisor(60).map(|p| p.label), Some("5#"));
        assert_eq!(highest_primorial_divisor(420).map(|p| p.label), Some("7#"));

        assert_eq!(format_primorial_multiple(1), None);
        assert_eq!(format_primorial_multiple(4), Some("2·2#".to_string()));
        assert_eq!(format_primorial_multiple(6), Some("3#".to_string()));
        assert_eq!(format_primorial_multiple(12), Some("2·3#".to_string()));
        assert_eq!(format_primorial_multiple(30), Some("5#".to_string()));
        assert_eq!(format_primorial_multiple(60), Some("2·5#".to_string()));
    }
}
